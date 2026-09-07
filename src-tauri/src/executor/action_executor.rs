//! Action sequence engine (spec §20, §30, §31).
//!
//! Execution contract:
//! 1. Resolve the target (cached HWND → verify; else enumerate + match).
//!    Ambiguity aborts with zero input injected.
//! 2. Restore + activate the target; activation MUST verify foreground.
//! 3. Run actions strictly in order, re-verifying focus before meaningful
//!    input steps and after long delays.
//! 4. Any verification or input failure aborts with a structured error.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::model::{
    Action, ExecutionOutcome, HistoryRecord, ScheduledTask, Settings, WindowCandidate,
};

use super::automation::PlatformAutomation;
use crate::platform::matching::resolve_target;

/// Longest delay after which focus is re-verified before continuing.
const FOCUS_RECHECK_DELAY_MS: u64 = 3000;

// ---------------------------------------------------------------------------
// Global execution serialization (D7)
// ---------------------------------------------------------------------------

static EXECUTION_HELD: AtomicBool = AtomicBool::new(false);

/// Held for the entire duration of one task's action sequence. A second task
/// attempting to acquire it surfaces `ExecutionAlreadyRunning` — sequences
/// never interleave.
pub struct ExecutionGuard {
    _private: (),
}

impl ExecutionGuard {
    pub fn try_acquire() -> AppResult<Self> {
        if EXECUTION_HELD.swap(true, Ordering::SeqCst) {
            return Err(AppError::ExecutionAlreadyRunning);
        }
        Ok(Self { _private: () })
    }
}

impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        EXECUTION_HELD.store(false, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// The resolved target context for one run.
struct ResolvedTarget {
    raw_hwnd: u64,
    description: String,
}

/// Resolve the task's window target: verify the cached HWND first, fall back
/// to full enumeration + matching. Updates `task.target.last_hwnd` on success.
fn resolve_target_for_task(
    platform: &dyn PlatformAutomation,
    task: &mut ScheduledTask,
) -> AppResult<ResolvedTarget> {
    // 1. Verify the cached HWND (HWNDs get invalidated/recycled — D4).
    if let Some(raw) = task.target.last_hwnd {
        if let Some(candidate) = platform.verify_cached(raw) {
            if crate::platform::matching::resolve_target(
                std::slice::from_ref(&candidate),
                &task.target,
            )
            .is_ok()
            {
                return Ok(ResolvedTarget {
                    description: describe(&candidate),
                    raw_hwnd: candidate.hwnd,
                });
            }
            // Cached window no longer matches criteria (title changed etc.);
            // fall through to full resolution.
        } else {
            task.target.last_hwnd = None;
        }
    }

    // 2. Full resolution from enumeration.
    let candidates = platform.enumerate();
    let candidate: WindowCandidate = resolve_target(&candidates, &task.target)?;
    task.target.last_hwnd = Some(candidate.hwnd);
    Ok(ResolvedTarget {
        description: describe(&candidate),
        raw_hwnd: candidate.hwnd,
    })
}

fn describe(candidate: &WindowCandidate) -> String {
    format!(
        "{} ({}, PID {})",
        candidate.title, candidate.process_name, candidate.process_id
    )
}

/// Execute a task's action sequence against the platform. The caller MUST
/// hold the [`ExecutionGuard`]. Returns a structured outcome suitable for
/// history.
pub fn execute_task(
    platform: &dyn PlatformAutomation,
    task: &mut ScheduledTask,
    settings: &Settings,
    notify: &dyn Fn(&str),
) -> ExecutionOutcome {
    match run_sequence(platform, task, settings, notify) {
        Ok(_) => ExecutionOutcome::Success,
        Err(e) => ExecutionOutcome::Failure {
            error_code: e.code().to_string(),
            error_message: e.to_string(),
        },
    }
}

/// Execute and also report the resolved target description ("" when
/// resolution failed before a target was identified).
fn execute_task_with_target(
    platform: &dyn PlatformAutomation,
    task: &mut ScheduledTask,
    settings: &Settings,
    notify: &dyn Fn(&str),
) -> (ExecutionOutcome, String) {
    match run_sequence(platform, task, settings, notify) {
        Ok(description) => (ExecutionOutcome::Success, description),
        Err(e) => (
            ExecutionOutcome::Failure {
                error_code: e.code().to_string(),
                error_message: e.to_string(),
            },
            task.target
                .last_hwnd
                .map(|_| "resolved target".to_string())
                .unwrap_or_else(|| "unresolved target".to_string()),
        ),
    }
}

fn run_sequence(
    platform: &dyn PlatformAutomation,
    task: &mut ScheduledTask,
    settings: &Settings,
    notify: &dyn Fn(&str),
) -> AppResult<String> {
    let target = resolve_target_for_task(platform, task)?;

    // Restore + activate + verify BEFORE any input (D6). Failure here means
    // no keyboard input has been sent yet.
    platform.restore(target.raw_hwnd);
    platform.activate(target.raw_hwnd)?;

    for action in &task.actions {
        // Focus re-verification before meaningful input steps (spec §30).
        if settings.abort_on_focus_loss
            && action_is_input(action)
            && !platform.is_foreground(target.raw_hwnd)
        {
            return Err(AppError::TargetLostFocus);
        }
        match action {
            Action::FocusTarget => platform.activate(target.raw_hwnd)?,
            Action::RestoreTarget => platform.restore(target.raw_hwnd),
            Action::TypeText { text } => platform.type_text(text)?,
            Action::PressKey {
                key,
                count,
                interval_ms,
            } => platform.press_key(*key, *count, *interval_ms)?,
            Action::KeyCombination { keys } => platform.press_combination(keys)?,
            Action::Delay { milliseconds } => {
                let started = Instant::now();
                let ms = (*milliseconds).min(u64::from(u32::MAX));
                std::thread::sleep(Duration::from_millis(ms));
                // Long delays: the user may have switched apps meanwhile.
                if settings.abort_on_focus_loss
                    && *milliseconds >= FOCUS_RECHECK_DELAY_MS
                    && !platform.is_foreground(target.raw_hwnd)
                {
                    return Err(AppError::TargetLostFocus);
                }
                let _ = started;
            }
            Action::Notify { message } => notify(message),
        }
    }
    Ok(target.description)
}

fn action_is_input(action: &Action) -> bool {
    matches!(
        action,
        Action::TypeText { .. } | Action::PressKey { .. } | Action::KeyCombination { .. }
    )
}

/// Build a history record for a finished run.
pub fn history_record(
    task: &ScheduledTask,
    scheduled_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
    target_description: &str,
    outcome: ExecutionOutcome,
) -> HistoryRecord {
    HistoryRecord {
        id: Uuid::new_v4(),
        task_id: task.id,
        task_name: task.name.clone(),
        scheduled_at,
        started_at,
        finished_at: Utc::now(),
        target_description: target_description.to_string(),
        outcome,
    }
}

/// Convenience wrapper used by the scheduler / `run_task_now`: acquire the
/// global guard, execute, and produce (record, resolved-target-description).
/// Returns `Err(AppError)` when the guard is held or resolution failed with
/// a *pre-input* error; otherwise returns the record with its outcome.
pub fn run_task_once(
    platform: &dyn PlatformAutomation,
    task: &mut ScheduledTask,
    settings: &Settings,
    scheduled_at: DateTime<Utc>,
    notify: &dyn Fn(&str),
) -> AppResult<HistoryRecord> {
    if !task.enabled {
        return Err(AppError::TaskDisabled);
    }
    let _guard = ExecutionGuard::try_acquire()?;
    let started_at = Utc::now();
    let (outcome, description) = execute_task_with_target(platform, task, settings, notify);
    Ok(history_record(
        task,
        scheduled_at,
        started_at,
        &description,
        outcome,
    ))
}

#[cfg(test)]
pub(crate) mod mock {
    //! Mock platform for executor tests: records events, never injects.

    use std::sync::Mutex as StdMutex;

    use super::*;
    use crate::model::Key;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MockEvent {
        Restore(u64),
        Activate(u64),
        TypeText(String),
        PressKey(Key, u32),
        Combination(Vec<Key>),
    }

    pub struct MockPlatform {
        pub events: StdMutex<Vec<MockEvent>>,
        /// Windows returned by `enumerate`.
        pub windows: Vec<WindowCandidate>,
        /// When true, `activate` fails (simulating focus refusal).
        pub fail_activation: bool,
        /// After N recorded events, `is_foreground` returns false
        /// (simulating the user switching away mid-sequence).
        pub lose_focus_after: Option<usize>,
        /// When true, input methods fail (simulating SendInput failure).
        pub fail_input: bool,
    }

    impl MockPlatform {
        pub fn new(windows: Vec<WindowCandidate>) -> Self {
            Self {
                events: StdMutex::new(Vec::new()),
                windows,
                fail_activation: false,
                lose_focus_after: None,
                fail_input: false,
            }
        }

        pub fn recorded(&self) -> Vec<MockEvent> {
            self.events.lock().expect("mock events").clone()
        }
    }

    impl PlatformAutomation for MockPlatform {
        fn enumerate(&self) -> Vec<WindowCandidate> {
            self.windows.clone()
        }

        fn verify_cached(&self, raw_hwnd: u64) -> Option<WindowCandidate> {
            self.windows.iter().find(|w| w.hwnd == raw_hwnd).cloned()
        }

        fn restore(&self, raw_hwnd: u64) {
            self.events
                .lock()
                .expect("mock events")
                .push(MockEvent::Restore(raw_hwnd));
        }

        fn activate(&self, raw_hwnd: u64) -> AppResult<()> {
            let mut events = self.events.lock().expect("mock events");
            if self.fail_activation {
                return Err(AppError::FailedToActivateTarget);
            }
            events.push(MockEvent::Activate(raw_hwnd));
            Ok(())
        }

        fn is_foreground(&self, _raw_hwnd: u64) -> bool {
            let count = self.events.lock().expect("mock events").len();
            match self.lose_focus_after {
                Some(after) => count <= after,
                None => true,
            }
        }

        fn type_text(&self, text: &str) -> AppResult<()> {
            if self.fail_input {
                return Err(AppError::InputInjectionFailed);
            }
            self.events
                .lock()
                .expect("mock events")
                .push(MockEvent::TypeText(text.to_string()));
            Ok(())
        }

        fn press_key(&self, key: Key, count: u32, _interval_ms: u64) -> AppResult<()> {
            if self.fail_input {
                return Err(AppError::InputInjectionFailed);
            }
            self.events
                .lock()
                .expect("mock events")
                .push(MockEvent::PressKey(key, count));
            Ok(())
        }

        fn press_combination(&self, keys: &[Key]) -> AppResult<()> {
            if self.fail_input {
                return Err(AppError::InputInjectionFailed);
            }
            self.events
                .lock()
                .expect("mock events")
                .push(MockEvent::Combination(keys.to_vec()));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::{MockEvent, MockPlatform};
    use super::*;
    use crate::model::{Key, MisfirePolicy, Schedule, TitleMatchMode, WindowTarget};

    fn terminal_window(hwnd: u64, title: &str) -> WindowCandidate {
        WindowCandidate {
            hwnd,
            title: title.into(),
            process_id: 18244,
            process_name: "WindowsTerminal.exe".into(),
            executable_path: None,
            is_visible: true,
        }
    }

    fn continue_task(target: WindowTarget, actions: Vec<Action>) -> ScheduledTask {
        ScheduledTask {
            id: Uuid::new_v4(),
            name: "Codex Continue".into(),
            enabled: true,
            target,
            schedule: Schedule::After {
                hours: 0,
                minutes: 0,
                seconds: 10,
            },
            actions,
            misfire_policy: MisfirePolicy::RunImmediately,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run_at: None,
            next_run_at: None,
        }
    }

    fn continue_actions() -> Vec<Action> {
        vec![
            Action::FocusTarget,
            Action::TypeText {
                text: "continue".into(),
            },
            Action::PressKey {
                key: Key::Enter,
                count: 1,
                interval_ms: 0,
            },
            Action::Delay {
                milliseconds: 0, // keep tests fast
            },
            Action::PressKey {
                key: Key::Enter,
                count: 1,
                interval_ms: 0,
            },
        ]
    }

    fn contains_target() -> WindowTarget {
        WindowTarget {
            title: Some("Codex".into()),
            title_match_mode: TitleMatchMode::Contains,
            ..Default::default()
        }
    }

    #[test]
    fn exact_action_ordering() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let platform = MockPlatform::new(windows);
        let mut task = continue_task(contains_target(), continue_actions());
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert_eq!(outcome, ExecutionOutcome::Success);
        assert_eq!(
            platform.recorded(),
            vec![
                MockEvent::Restore(42),
                MockEvent::Activate(42),
                MockEvent::Activate(42),
                MockEvent::TypeText("continue".into()),
                MockEvent::PressKey(Key::Enter, 1),
                MockEvent::PressKey(Key::Enter, 1),
            ]
        );
        // Resolution must cache the HWND for next time (D4).
        assert_eq!(task.target.last_hwnd, Some(42));
    }

    #[test]
    fn cached_hwnd_is_verified_first_without_enumeration() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let platform = MockPlatform::new(windows);
        let mut task = continue_task(contains_target(), vec![Action::FocusTarget]);
        task.target.last_hwnd = Some(42);
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert_eq!(outcome, ExecutionOutcome::Success);
        // No Resolve event is recorded by the mock; the important part is
        // success without enumerate being observable + cached hwnd retained.
        assert_eq!(task.target.last_hwnd, Some(42));
    }

    #[test]
    fn activation_failure_sends_no_input() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let mut platform = MockPlatform::new(windows);
        platform.fail_activation = true;
        let mut task = continue_task(contains_target(), continue_actions());
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert_eq!(
            outcome,
            ExecutionOutcome::Failure {
                error_code: "FailedToActivateTarget".into(),
                error_message: "target window could not be activated; no keyboard input was sent"
                    .into(),
            }
        );
        // No TypeText/PressKey events at all.
        assert!(
            platform
                .recorded()
                .iter()
                .all(|e| !matches!(e, MockEvent::TypeText(_) | MockEvent::PressKey(_, _)))
        );
    }

    #[test]
    fn ambiguous_target_aborts_without_input() {
        let windows = vec![
            terminal_window(1, "Codex — alpha"),
            terminal_window(2, "Codex — beta"),
        ];
        let platform = MockPlatform::new(windows);
        let mut task = continue_task(contains_target(), continue_actions());
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert_eq!(
            outcome,
            ExecutionOutcome::Failure {
                error_code: "TargetAmbiguous".into(),
                error_message: "multiple windows matched the target criteria; refine the matching"
                    .into(),
            }
        );
        assert!(platform.recorded().is_empty());
    }

    #[test]
    fn target_not_found_aborts_without_input() {
        let windows = vec![terminal_window(1, "Totally unrelated")];
        let platform = MockPlatform::new(windows);
        let mut task = continue_task(contains_target(), continue_actions());
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert!(matches!(
            outcome,
            ExecutionOutcome::Failure { error_code, .. } if error_code == "TargetNotFound"
        ));
        assert!(platform.recorded().is_empty());
    }

    #[test]
    fn focus_loss_mid_sequence_aborts_before_next_input() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let mut platform = MockPlatform::new(windows);
        // Lose focus AFTER the first Enter: 4 events recorded by then
        // (restore, activate, focus-action, type); the pre-input focus
        // check for the second Enter happens when 5 events exist.
        platform.lose_focus_after = Some(4);
        let mut task = continue_task(contains_target(), continue_actions());
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert!(matches!(
            outcome,
            ExecutionOutcome::Failure { error_code, .. } if error_code == "TargetLostFocus"
        ));
        let recorded = platform.recorded();
        // The second Enter must NOT have been sent.
        assert_eq!(
            recorded
                .iter()
                .filter(|e| matches!(e, MockEvent::PressKey(Key::Enter, _)))
                .count(),
            1
        );
    }

    #[test]
    fn focus_loss_can_be_disabled_by_setting() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let mut platform = MockPlatform::new(windows);
        platform.lose_focus_after = Some(3);
        let mut task = continue_task(contains_target(), continue_actions());
        let settings = Settings {
            abort_on_focus_loss: false,
            ..Default::default()
        };

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert_eq!(outcome, ExecutionOutcome::Success);
    }

    #[test]
    fn input_failure_is_structured() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let mut platform = MockPlatform::new(windows);
        platform.fail_input = true;
        let mut task = continue_task(contains_target(), continue_actions());
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert!(matches!(
            outcome,
            ExecutionOutcome::Failure { error_code, .. } if error_code == "InputInjectionFailed"
        ));
    }

    #[test]
    fn key_combination_reaches_platform() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let platform = MockPlatform::new(windows);
        let actions = vec![
            Action::FocusTarget,
            Action::KeyCombination {
                keys: vec![Key::Ctrl, Key::Letter('c')],
            },
        ];
        let mut task = continue_task(contains_target(), actions);
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert_eq!(outcome, ExecutionOutcome::Success);
        assert!(
            platform
                .recorded()
                .contains(&MockEvent::Combination(vec![Key::Ctrl, Key::Letter('c')]))
        );
    }

    #[test]
    fn notify_action_invokes_callback() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        let platform = MockPlatform::new(windows);
        let actions = vec![
            Action::FocusTarget,
            Action::Notify {
                message: "hello".into(),
            },
        ];
        let mut task = continue_task(contains_target(), actions);
        let settings = Settings::default();

        let received = std::sync::Mutex::new(Vec::<String>::new());
        let outcome = execute_task(&platform, &mut task, &settings, &|m| {
            received.lock().expect("test mutex").push(m.to_string());
        });
        assert_eq!(outcome, ExecutionOutcome::Success);
        assert_eq!(
            *received.lock().expect("test mutex"),
            vec!["hello".to_string()]
        );
    }

    #[test]
    fn execution_is_globally_serialized() {
        // While one guard is held, a second acquisition fails with the
        // structured error; dropping the first releases.
        let _first = ExecutionGuard::try_acquire().expect("first guard");
        assert!(matches!(
            ExecutionGuard::try_acquire(),
            Err(AppError::ExecutionAlreadyRunning)
        ));
        drop(_first);
        assert!(ExecutionGuard::try_acquire().is_ok());
    }

    #[test]
    fn long_delay_rechecks_focus() {
        let windows = vec![terminal_window(42, "Codex — rust-reality")];
        // Lose focus after activation (2 events: restore + activate).
        let mut platform = MockPlatform::new(windows);
        platform.lose_focus_after = Some(2);
        let actions = vec![
            Action::FocusTarget,
            Action::Delay {
                milliseconds: FOCUS_RECHECK_DELAY_MS,
            },
        ];
        let mut task = continue_task(contains_target(), actions);
        let settings = Settings::default();

        let outcome = execute_task(&platform, &mut task, &settings, &|_| {});
        assert!(matches!(
            outcome,
            ExecutionOutcome::Failure { error_code, .. } if error_code == "TargetLostFocus"
        ));
    }
}
