//! Shared state. Every mutation commits to disk before publishing the in-memory
//! document, under the same transaction gate used by scheduler reconciliation.
use crate::error::{AppError, AppResult};
use crate::executor::{self, PlatformAutomation};
use crate::model::{ExecutionOutcome, HistoryRecord, ScheduledTask};
use crate::scheduler::{Scheduler, SystemClock};
use crate::store::{JsonStore, StoreDocument, bound_history};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub mod events {
    pub const TASK_CREATED: &str = "task-created";
    pub const TASK_UPDATED: &str = "task-updated";
    pub const TASK_STARTED: &str = "task-started";
    pub const TASK_COMPLETED: &str = "task-completed";
    pub const TASK_FAILED: &str = "task-failed";
    pub const SCHEDULER_UPDATED: &str = "scheduler-updated";
    pub const EXECUTION_NOTIFY: &str = "execution-notify";
}

pub struct AppState {
    pub doc: Mutex<StoreDocument>,
    pub store: JsonStore,
    pub scheduler: Arc<Scheduler>,
    pub automation: Arc<dyn PlatformAutomation>,
}

impl AppState {
    pub fn new(
        store: JsonStore,
        automation: Arc<dyn PlatformAutomation>,
        app: AppHandle,
    ) -> AppResult<Arc<Self>> {
        // Do not reconcile here: it consumes due occurrences before the live
        // scheduler can dispatch them. Start only after Tauri manages this state.
        let doc = store.load()?;
        Ok(Arc::new(Self {
            doc: Mutex::new(doc),
            store,
            automation,
            scheduler: Scheduler::new(
                Arc::new(SystemClock),
                {
                    let app = app.clone();
                    Arc::new(move |task| Self::fire_task(app.clone(), task))
                },
                {
                    let app = app.clone();
                    Arc::new(move |tasks| {
                        let state: tauri::State<Arc<AppState>> = app.state();
                        // The scheduler already owns its transaction gate.
                        let mut doc = state.lock_doc()?;
                        let mut next = doc.clone();
                        next.tasks = tasks.to_vec();
                        state.store.save(&next)?;
                        *doc = next;
                        let _ = app.emit(events::SCHEDULER_UPDATED, tasks);
                        Ok(())
                    })
                },
            ),
        }))
    }

    pub fn lock_doc(&self) -> AppResult<MutexGuard<'_, StoreDocument>> {
        self.doc
            .lock()
            .map_err(|e| AppError::StateUnavailable(e.to_string()))
    }

    pub fn update_doc<T>(
        &self,
        change: impl FnOnce(&mut StoreDocument) -> AppResult<T>,
    ) -> AppResult<T> {
        let _transaction = self.scheduler.transaction()?;
        let mut doc = self.lock_doc()?;
        let mut next = doc.clone();
        let result = change(&mut next)?;
        self.store.save(&next)?;
        self.scheduler.replace_tasks(next.tasks.clone());
        *doc = next;
        Ok(result)
    }

    pub fn sync_scheduler(&self) -> AppResult<()> {
        let _transaction = self.scheduler.transaction()?;
        self.scheduler.replace_tasks(self.lock_doc()?.tasks.clone());
        Ok(())
    }

    fn fire_task(app: AppHandle, task: ScheduledTask) {
        let failure_task = task.clone();
        let failure_app = app.clone();
        if let Err(error) = std::thread::Builder::new()
            .name(format!("exec-{}", task.id))
            .spawn(move || {
                let state: tauri::State<Arc<AppState>> = app.state();
                if let Err(error) = state.execute_and_record(&app, task) {
                    eprintln!("execution failed: {error}");
                    let _ = app.emit(events::TASK_FAILED, &error);
                }
            })
        {
            let now = chrono::Utc::now();
            let record = executor::history_record(
                &failure_task,
                failure_task.next_run_at.unwrap_or(now),
                now,
                "unresolved target",
                ExecutionOutcome::Failure {
                    error_code: "StateUnavailable".into(),
                    error_message: error.to_string(),
                },
            );
            // Scheduler owns the transaction gate here; report without re-entering it.
            eprintln!("could not start execution worker: {error}");
            let _ = failure_app.emit(events::TASK_FAILED, &record);
        }
    }

    pub fn execute_and_record(
        &self,
        app: &AppHandle,
        mut task: ScheduledTask,
    ) -> AppResult<HistoryRecord> {
        let settings = self.lock_doc()?.settings.clone();
        let started_at = chrono::Utc::now();
        let scheduled_at = task.next_run_at.unwrap_or(started_at);
        let _ = app.emit(events::TASK_STARTED, task.id);
        let task_id = task.id;
        let task_version = task.updated_at;
        let cancelled = || {
            self.scheduler.is_paused()
                || self.lock_doc().map_or(true, |doc| {
                    !doc.tasks
                        .iter()
                        .any(|t| t.id == task_id && t.enabled && t.updated_at == task_version)
                })
        };
        let record = match executor::run_task_once_cancellable(
            self.automation.as_ref(),
            &mut task,
            &settings,
            scheduled_at,
            &|message| {
                let _ = app.emit(events::EXECUTION_NOTIFY, message);
                let _ = app
                    .notification()
                    .builder()
                    .title("Agent Pulse")
                    .body(message)
                    .show();
            },
            &cancelled,
        ) {
            Ok(record) => record,
            Err(error) => executor::history_record(
                &task,
                scheduled_at,
                started_at,
                "unresolved target",
                ExecutionOutcome::Failure {
                    error_code: error.code().into(),
                    error_message: error.to_string(),
                },
            ),
        };
        self.record_execution(&record)?;
        let success = matches!(record.outcome, ExecutionOutcome::Success);
        if (success && settings.notify_on_success) || (!success && settings.notify_on_failure) {
            let body = match &record.outcome {
                ExecutionOutcome::Success => format!("{}: keyboard sequence sent.", task.name),
                ExecutionOutcome::Failure { error_message, .. } => {
                    format!("{}: {error_message}", task.name)
                }
            };
            let _ = app
                .notification()
                .builder()
                .title("Agent Pulse")
                .body(body)
                .show();
        }
        let _ = app.emit(
            if success {
                events::TASK_COMPLETED
            } else {
                events::TASK_FAILED
            },
            &record,
        );
        Ok(record)
    }

    fn record_execution(&self, record: &HistoryRecord) -> AppResult<()> {
        self.update_doc(|doc| {
            // Never restore an execution snapshot over a task edited/deleted during
            // a delay, or re-arm an occurrence that the scheduler already consumed.
            if let Some(task) = doc.tasks.iter_mut().find(|task| task.id == record.task_id) {
                task.last_run_at = Some(
                    task.last_run_at
                        .map_or(record.started_at, |last| last.max(record.started_at)),
                );
            }
            doc.history.push(record.clone());
            bound_history(&mut doc.history, doc.settings.history_limit);
            Ok(())
        })
    }

    pub fn emit_task_event(&self, app: &AppHandle, event: &str, task: &ScheduledTask) {
        let _ = app.emit(event, task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::action_executor::mock::MockPlatform;
    use crate::model::{Action, MisfirePolicy, Schedule, Theme, WindowTarget};

    fn state() -> (AppState, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("agent-pulse-state-{}", uuid::Uuid::new_v4()));
        let state = AppState {
            doc: Mutex::new(StoreDocument::default()),
            store: JsonStore::new(path.join("store.json")),
            scheduler: Scheduler::new(
                Arc::new(SystemClock),
                Arc::new(|_| {}),
                Arc::new(|_| Ok(())),
            ),
            automation: Arc::new(MockPlatform::new(vec![])),
        };
        (state, path)
    }

    #[test]
    fn settings_and_history_changes_update_memory_and_disk_together() {
        let (state, dir) = state();
        state
            .update_doc(|doc| {
                doc.settings.theme = Theme::Dark;
                Ok(())
            })
            .expect("update");
        assert_eq!(state.lock_doc().expect("lock").settings.theme, Theme::Dark);
        assert_eq!(
            state.store.load().expect("load").settings.theme,
            Theme::Dark
        );
        let task = ScheduledTask::new(
            "test",
            WindowTarget::default(),
            Schedule::Every {
                interval_seconds: 5,
            },
            vec![Action::FocusTarget],
            MisfirePolicy::default(),
            chrono::Utc::now(),
        )
        .expect("task");
        let now = chrono::Utc::now();
        let record = executor::history_record(&task, now, now, "test", ExecutionOutcome::Success);
        state.record_execution(&record).expect("record");
        state
            .update_doc(|doc| {
                doc.history.clear();
                Ok(())
            })
            .expect("clear");
        assert!(state.lock_doc().expect("lock").history.is_empty());
        assert!(state.store.load().expect("load").history.is_empty());
        std::fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn failed_save_does_not_publish_memory_changes() {
        let (state, dir) = state();
        std::fs::write(&dir, "parent is a file").expect("create obstacle");
        assert!(
            state
                .update_doc(|doc| {
                    doc.settings.theme = Theme::Dark;
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(
            state.lock_doc().expect("lock").settings.theme,
            Theme::System
        );
        std::fs::remove_file(dir).expect("cleanup");
    }

    #[test]
    fn execution_completion_preserves_newer_edits_and_consumed_deadline() {
        let (state, dir) = state();
        let mut task = ScheduledTask::new(
            "original",
            WindowTarget::default(),
            Schedule::Every {
                interval_seconds: 5,
            },
            vec![Action::FocusTarget],
            MisfirePolicy::default(),
            chrono::Utc::now(),
        )
        .expect("task");
        let now = chrono::Utc::now();
        let record = executor::history_record(&task, now, now, "test", ExecutionOutcome::Success);
        task.name = "edited while running".into();
        task.next_run_at = None;
        state
            .update_doc(|doc| {
                doc.tasks.push(task.clone());
                Ok(())
            })
            .expect("edit");
        state.record_execution(&record).expect("record");
        let saved = state.store.load().expect("load");
        assert_eq!(saved.tasks[0].name, "edited while running");
        assert_eq!(saved.tasks[0].next_run_at, None);
        std::fs::remove_dir_all(dir).expect("cleanup");
    }
}
