//! Scheduler core (spec §18–§19). Scheduling lives in Rust; the frontend's
//! countdowns are display-only (docs/DECISIONS.md D1).
//!
//! The engine waits on a condvar until the nearest `next_run_at` (or until a
//! task change / shutdown wakes it) — no busy loop, no polling.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::error::{AppError, AppResult};
use crate::model::{MisfirePolicy, Schedule, ScheduledTask};

use super::clock::Clock;

/// Longest the engine sleeps when no task is scheduled (periodic reconcile
/// against wall-clock changes such as system clock adjustments).
const IDLE_RECONCILE: Duration = Duration::from_secs(1);

/// Outcome computed for one task during a reconcile pass.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Task is enabled with a future `next_run_at`; keep waiting.
    Wait,
    /// Task is disabled; never fires.
    Disabled,
    /// Overdue one-shot with `Skip`: cancel it (no fire, clear `next_run_at`).
    SkipOverdue,
    /// Fire now.
    Fire,
}

/// Pure reconciliation of a task against `now` — the heart of misfire
/// handling and sleep/restart recovery.
pub fn decide(task: &ScheduledTask, now: DateTime<Utc>) -> Decision {
    if !task.enabled {
        return Decision::Disabled;
    }
    let Some(next) = task.next_run_at else {
        return Decision::Wait; // one-shot already fired, or not armed
    };
    if next > now {
        return Decision::Wait;
    }
    // Overdue: apply the misfire policy (spec §19).
    match task.misfire_policy {
        MisfirePolicy::Skip if now.signed_duration_since(next) > chrono::Duration::seconds(2) => {
            Decision::SkipOverdue
        }
        MisfirePolicy::Skip => Decision::Fire,
        MisfirePolicy::RunImmediately => Decision::Fire,
    }
}

/// Compute the next `next_run_at` after a task fired at (or was scheduled
/// for) `scheduled`, given the real current time `now`.
///
/// * one-shot schedules → `None` (done)
/// * recurring → next whole interval slot past `now`, computed from the
///   *scheduled* time so drift never accumulates
pub fn compute_next_run(
    task: &ScheduledTask,
    scheduled: DateTime<Utc>,
    now: DateTime<Utc>,
) -> AppResult<Option<DateTime<Utc>>> {
    match task.schedule {
        Schedule::After { .. } | Schedule::At { .. } => Ok(None),
        Schedule::Every { .. } => task.schedule.next_after(scheduled, now).map(Some),
    }
}

/// What the engine observes after a reconcile pass.
#[derive(Debug, Default, Clone)]
pub struct ReconcileReport {
    /// Tasks to fire now (id + name snapshot taken at decision time).
    pub fire: Vec<ScheduledTask>,
    /// One-shot tasks cancelled by `Skip`.
    pub skipped: Vec<ScheduledTask>,
}

/// Pure reconcile over a whole task list. Mutates tasks that fire or skip
/// (updating `last_run_at` / `next_run_at`); the caller decides persistence.
pub fn reconcile(tasks: &mut [ScheduledTask], now: DateTime<Utc>) -> ReconcileReport {
    let mut report = ReconcileReport::default();
    for task in tasks.iter_mut() {
        match decide(task, now) {
            Decision::Wait | Decision::Disabled => {}
            Decision::SkipOverdue => {
                // Jump recurring tasks to their next future slot; clear
                // one-shot schedules entirely.
                let Some(scheduled) = task.next_run_at else {
                    continue;
                };
                task.next_run_at = compute_next_run(task, scheduled, now).ok().flatten();
                report.skipped.push(task.clone());
            }
            Decision::Fire => {
                let Some(scheduled) = task.next_run_at else {
                    continue;
                };
                // Preserve the actual due slot in the execution snapshot.
                report.fire.push(task.clone());
                task.last_run_at = Some(now);
                task.next_run_at = compute_next_run(task, scheduled, now).ok().flatten();
            }
        }
    }
    report
}

type FireCallback = dyn Fn(ScheduledTask) + Send + Sync;
type UpdateCallback = dyn Fn(&[ScheduledTask]) -> AppResult<()> + Send + Sync;

struct Inner {
    tasks: Vec<ScheduledTask>,
    paused: bool,
    running: bool,
}

/// The live scheduler. Owns a dedicated thread; task mutations go through
/// `replace_tasks` which wakes the loop immediately.
pub struct Scheduler {
    /// Serializes command transactions with reconcile + persistence. Always
    /// acquire before doc/inner; callbacks must not re-enter transactions.
    transaction: Mutex<()>,
    inner: Arc<(Mutex<Inner>, Condvar)>,
    clock: Arc<dyn Clock>,
    on_fire: Arc<FireCallback>,
    /// Callback fired when task state changed after a pass (persist + emit).
    on_update: Arc<UpdateCallback>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Scheduler {
    /// Create a scheduler. `on_fire` runs when a task is due — it should
    /// hand execution to the executor rather than blocking this loop.
    /// `on_update` receives the full (mutated) task list after each pass that
    /// changed state, for persistence and UI events.
    pub fn new(
        clock: Arc<dyn Clock>,
        on_fire: Arc<FireCallback>,
        on_update: Arc<UpdateCallback>,
    ) -> Arc<Self> {
        Arc::new(Self {
            transaction: Mutex::new(()),
            inner: Arc::new((
                Mutex::new(Inner {
                    tasks: Vec::new(),
                    paused: false,
                    running: true,
                }),
                Condvar::new(),
            )),
            clock,
            on_fire,
            on_update,
            thread: Mutex::new(None),
        })
    }

    pub fn transaction(&self) -> AppResult<std::sync::MutexGuard<'_, ()>> {
        self.transaction
            .lock()
            .map_err(|e| AppError::StateUnavailable(e.to_string()))
    }

    /// Replace the managed task list and wake the loop.
    pub fn replace_tasks(&self, tasks: Vec<ScheduledTask>) {
        let (lock, cvar) = &*self.inner;
        let mut inner = lock.lock().unwrap_or_else(|e| e.into_inner());
        inner.tasks = tasks;
        cvar.notify_all();
    }

    /// Pause/resume firing (tray "Pause All"). Pausing only stops firing;
    /// `next_run_at` values are retained.
    pub fn set_paused(&self, paused: bool) {
        let (lock, cvar) = &*self.inner;
        let mut inner = lock.lock().unwrap_or_else(|e| e.into_inner());
        inner.paused = paused;
        cvar.notify_all();
    }

    pub fn is_paused(&self) -> bool {
        self.inner
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .paused
    }

    /// Start the scheduler thread.
    pub fn start(self: &Arc<Self>) -> AppResult<()> {
        let me = Arc::clone(self);
        let handle = std::thread::Builder::new()
            .name("agent-pulse-scheduler".into())
            .spawn(move || me.run_loop())
            .map_err(|e| AppError::StateUnavailable(format!("spawn scheduler: {e}")))?;
        *self.thread.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
        Ok(())
    }

    /// Signal shutdown and join the thread.
    pub fn stop(&self) {
        {
            let (lock, cvar) = &*self.inner;
            let mut inner = lock.lock().unwrap_or_else(|e| e.into_inner());
            inner.running = false;
            cvar.notify_all();
        }
        if let Some(handle) = self.thread.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = handle.join();
        }
    }

    fn run_loop(self: Arc<Self>) {
        let (lock, cvar) = &*self.inner;
        loop {
            let Ok(transaction) = self.transaction() else {
                return;
            };
            let mut inner = lock.lock().unwrap_or_else(|e| e.into_inner());
            if !inner.running {
                return;
            }
            let now = self.clock.now();
            if !inner.paused {
                let mut tasks = inner.tasks.clone();
                let report = reconcile(&mut tasks, now);
                if !report.fire.is_empty() || !report.skipped.is_empty() {
                    drop(inner);
                    // Persist BEFORE emitting execution. A failed save leaves the
                    // occurrence armed, so the next pass can retry without typing.
                    let persisted = (self.on_update)(&tasks);
                    inner = lock.lock().unwrap_or_else(|e| e.into_inner());
                    if persisted.is_ok() {
                        inner.tasks = tasks;
                        drop(inner);
                        for task in report.fire {
                            (self.on_fire)(task);
                        }
                        inner = lock.lock().unwrap_or_else(|e| e.into_inner());
                    } else if let Err(error) = persisted {
                        eprintln!("scheduler persistence failed: {error}");
                    }
                }
            }
            // Bound wall-clock reconciliation after resume or clock changes.
            let wait = if inner.paused {
                IDLE_RECONCILE
            } else {
                inner
                    .tasks
                    .iter()
                    .filter(|t| t.enabled)
                    .filter_map(|t| t.next_run_at)
                    .min()
                    .map(|at| (at - self.clock.now()).to_std().unwrap_or(IDLE_RECONCILE))
                    .map(|d| d.min(IDLE_RECONCILE))
                    .unwrap_or(IDLE_RECONCILE)
            };
            drop(transaction);
            let (guard, _) = cvar
                .wait_timeout(inner, wait)
                .unwrap_or_else(|e| e.into_inner());
            drop(guard);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::model::{Action, Schedule, WindowTarget};

    fn utc(secs: i64) -> DateTime<Utc> {
        chrono::TimeZone::timestamp_opt(&Utc, secs, 0)
            .single()
            .expect("test ts")
    }

    fn task(
        name: &str,
        schedule: Schedule,
        policy: MisfirePolicy,
        next_run_at: Option<DateTime<Utc>>,
        enabled: bool,
    ) -> ScheduledTask {
        ScheduledTask {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            enabled,
            target: WindowTarget::default(),
            schedule,
            actions: vec![Action::FocusTarget],
            misfire_policy: policy,
            created_at: utc(0),
            updated_at: utc(0),
            last_run_at: None,
            next_run_at,
        }
    }

    // --- decide() ---

    #[test]
    fn disabled_task_never_fires() {
        let t = task(
            "d",
            Schedule::After {
                hours: 0,
                minutes: 0,
                seconds: 1,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(1)),
            false,
        );
        assert_eq!(decide(&t, utc(100)), Decision::Disabled);
    }

    #[test]
    fn future_task_waits() {
        let t = task(
            "w",
            Schedule::After {
                hours: 1,
                minutes: 0,
                seconds: 0,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(200)),
            true,
        );
        assert_eq!(decide(&t, utc(100)), Decision::Wait);
    }

    #[test]
    fn overdue_run_immediately_fires() {
        let t = task(
            "f",
            Schedule::After {
                hours: 1,
                minutes: 0,
                seconds: 0,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(50)),
            true,
        );
        assert_eq!(decide(&t, utc(100)), Decision::Fire);
    }

    #[test]
    fn overdue_skip_cancels_one_shot() {
        let t = task(
            "s",
            Schedule::After {
                hours: 1,
                minutes: 0,
                seconds: 0,
            },
            MisfirePolicy::Skip,
            Some(utc(50)),
            true,
        );
        assert_eq!(decide(&t, utc(100)), Decision::SkipOverdue);
    }

    #[test]
    fn overdue_skip_cancels_recurring_occurrence() {
        let t = task(
            "s",
            Schedule::Every {
                interval_seconds: 60,
            },
            MisfirePolicy::Skip,
            Some(utc(50)),
            true,
        );
        assert_eq!(decide(&t, utc(200)), Decision::SkipOverdue);
    }

    // --- compute_next_run() ---

    #[test]
    fn one_shot_has_no_next_run() {
        let t = task(
            "o",
            Schedule::After {
                hours: 1,
                minutes: 0,
                seconds: 0,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(60)),
            true,
        );
        assert_eq!(compute_next_run(&t, utc(60), utc(60)).expect("next"), None);
    }

    #[test]
    fn recurring_next_run_is_drift_free_after_large_jump() {
        let t = task(
            "r",
            Schedule::Every {
                interval_seconds: 60,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(0)),
            true,
        );
        // Sleep-like jump: now is 10+ intervals past the scheduled time.
        let next = compute_next_run(&t, utc(0), utc(601))
            .expect("next")
            .expect("recurring");
        assert_eq!(next, utc(660));
    }

    #[test]
    fn recurring_next_run_after_small_lateness() {
        let t = task(
            "r",
            Schedule::Every {
                interval_seconds: 60,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(0)),
            true,
        );
        let next = compute_next_run(&t, utc(0), utc(61))
            .expect("next")
            .expect("recurring");
        assert_eq!(next, utc(120));
    }

    // --- reconcile() ---

    #[test]
    fn reconcile_fires_and_updates_state() {
        let mut tasks = vec![task(
            "r",
            Schedule::Every {
                interval_seconds: 60,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(60)),
            true,
        )];
        let report = reconcile(&mut tasks, utc(125));
        assert_eq!(report.fire.len(), 1);
        assert_eq!(report.fire[0].next_run_at, Some(utc(60)));
        let t = &tasks[0];
        assert_eq!(t.last_run_at, Some(utc(125)));
        assert_eq!(t.next_run_at, Some(utc(180)));
    }

    #[test]
    fn skip_policy_runs_on_time_but_skips_missed_deadlines() {
        let t = task(
            "skip",
            Schedule::Every {
                interval_seconds: 60,
            },
            MisfirePolicy::Skip,
            Some(utc(60)),
            true,
        );
        assert_eq!(decide(&t, utc(60)), Decision::Fire);
        assert_eq!(decide(&t, utc(61)), Decision::Fire);
        assert_eq!(decide(&t, utc(63)), Decision::SkipOverdue);
    }

    #[test]
    fn skip_only_live_engine_persists_without_firing() {
        let (tx, rx) = std::sync::mpsc::channel();
        let scheduler = Scheduler::new(
            super::super::clock::test_clock::TestClock::at(2000),
            Arc::new(|_| panic!("skipped task must not fire")),
            Arc::new(move |tasks| {
                let _ = tx.send(tasks[0].next_run_at);
                Ok(())
            }),
        );
        scheduler.replace_tasks(vec![task(
            "skip",
            Schedule::After {
                hours: 0,
                minutes: 0,
                seconds: 1,
            },
            MisfirePolicy::Skip,
            Some(utc(60)),
            true,
        )]);
        scheduler.start().expect("start");
        let received = rx.recv_timeout(Duration::from_secs(2));
        scheduler.stop();
        assert_eq!(received.expect("updated skipped task"), None);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn reconcile_skips_overdue_and_recomputes() {
        let mut tasks = vec![task(
            "s",
            Schedule::Every {
                interval_seconds: 60,
            },
            MisfirePolicy::Skip,
            Some(utc(60)),
            true,
        )];
        let report = reconcile(&mut tasks, utc(70));
        assert!(report.fire.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(tasks[0].next_run_at, Some(utc(120)));
        assert_eq!(tasks[0].last_run_at, None);
    }

    #[test]
    fn reconcile_restart_recovery_one_shot_run_immediately() {
        // Task scheduled before a restart; on startup it fires immediately.
        let mut tasks = vec![task(
            "restart",
            Schedule::After {
                hours: 5,
                minutes: 5,
                seconds: 0,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(1000)),
            true,
        )];
        let report = reconcile(&mut tasks, utc(5000));
        assert_eq!(report.fire.len(), 1);
        assert_eq!(tasks[0].next_run_at, None);
    }

    #[test]
    fn reconcile_sleep_like_jump() {
        // Recurring every 30 min; machine slept 12 h past the slot.
        let mut tasks = vec![task(
            "sleep",
            Schedule::Every {
                interval_seconds: 1800,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(100)),
            true,
        )];
        let now = utc(100 + 12 * 3600);
        let report = reconcile(&mut tasks, now);
        assert_eq!(report.fire.len(), 1);
        let next = tasks[0].next_run_at.expect("recurring next");
        assert!(next > now);
        // Aligned to the original grid: (next - 100) % 1800 == 0
        assert_eq!((next.timestamp() - 100) % 1800, 0);
    }

    #[test]
    fn reconcile_ignores_disabled_and_future() {
        let mut tasks = vec![
            task(
                "d",
                Schedule::After {
                    hours: 1,
                    minutes: 0,
                    seconds: 0,
                },
                MisfirePolicy::RunImmediately,
                Some(utc(1)),
                false,
            ),
            task(
                "f",
                Schedule::After {
                    hours: 1,
                    minutes: 0,
                    seconds: 0,
                },
                MisfirePolicy::RunImmediately,
                Some(utc(500)),
                true,
            ),
        ];
        let report = reconcile(&mut tasks, utc(100));
        assert!(report.fire.is_empty());
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn one_shot_fire_clears_next_run() {
        let mut tasks = vec![task(
            "one",
            Schedule::After {
                hours: 0,
                minutes: 1,
                seconds: 0,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(60)),
            true,
        )];
        let report = reconcile(&mut tasks, utc(61));
        assert_eq!(report.fire.len(), 1);
        assert_eq!(tasks[0].next_run_at, None);
    }

    // --- schedule validation surfaced through task creation ---

    #[test]
    fn invalid_schedule_is_rejected() {
        let result = ScheduledTask::new(
            "bad",
            WindowTarget::default(),
            Schedule::Every {
                interval_seconds: 0,
            },
            vec![],
            MisfirePolicy::default(),
            utc(0),
        );
        assert!(matches!(result, Err(AppError::InvalidSchedule(_))));
    }

    // --- live engine thread ---

    #[test]
    fn engine_thread_fires_overdue_task_and_reports_update() {
        let clock = super::super::clock::test_clock::TestClock::at(2000);
        let (fire_tx, fire_rx) = std::sync::mpsc::channel::<ScheduledTask>();
        let (update_tx, update_rx) = std::sync::mpsc::channel::<usize>();

        let scheduler = Scheduler::new(
            clock,
            Arc::new(move |t| {
                let _ = fire_tx.send(t);
            }),
            Arc::new(move |tasks| {
                let _ = update_tx.send(tasks.len());
                Ok(())
            }),
        );
        // Overdue one-shot scheduled long before "now" (restart recovery).
        scheduler.replace_tasks(vec![task(
            "live",
            Schedule::After {
                hours: 1,
                minutes: 0,
                seconds: 0,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(100)),
            true,
        )]);
        scheduler.start().expect("start");
        let fired = fire_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("task should fire promptly");
        assert_eq!(fired.name, "live");
        let len = update_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("update should be reported");
        assert_eq!(len, 1);
        scheduler.stop();
    }

    #[test]
    fn paused_engine_does_not_fire() {
        let clock = super::super::clock::test_clock::TestClock::at(2000);
        let (fire_tx, fire_rx) = std::sync::mpsc::channel::<ScheduledTask>();
        let scheduler = Scheduler::new(
            clock,
            Arc::new(move |t| {
                let _ = fire_tx.send(t);
            }),
            Arc::new(|_| Ok(())),
        );
        scheduler.set_paused(true);
        scheduler.replace_tasks(vec![task(
            "paused",
            Schedule::After {
                hours: 1,
                minutes: 0,
                seconds: 0,
            },
            MisfirePolicy::RunImmediately,
            Some(utc(100)),
            true,
        )]);
        scheduler.start().expect("start");
        assert!(fire_rx.recv_timeout(Duration::from_millis(300)).is_err());
        scheduler.stop();
    }
}
