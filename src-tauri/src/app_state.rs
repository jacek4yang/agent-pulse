//! Shared application state: the store, scheduler, and automation platform
//! wired together. Rust is the single source of truth (spec §43).

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppResult;
use crate::executor::{self, PlatformAutomation};
use crate::model::{ExecutionOutcome, ScheduledTask};
use crate::scheduler::{Clock, Scheduler, SystemClock};
use crate::store::{JsonStore, StoreDocument, bound_history};

/// Events emitted to the frontend (spec §44).
pub mod events {
    pub const TASK_CREATED: &str = "task-created";
    pub const TASK_UPDATED: &str = "task-updated";
    pub const TASK_STARTED: &str = "task-started";
    pub const TASK_COMPLETED: &str = "task-completed";
    pub const TASK_FAILED: &str = "task-failed";
    pub const SCHEDULER_UPDATED: &str = "scheduler-updated";
    /// In-sequence notification from a `Notify` action.
    pub const EXECUTION_NOTIFY: &str = "execution-notify";
}

/// Application-wide shared state managed by Tauri.
pub struct AppState {
    /// In-memory mirror of the persisted document; every mutation persists.
    pub doc: Mutex<StoreDocument>,
    pub store: JsonStore,
    pub scheduler: Arc<Scheduler>,
    pub automation: Arc<dyn PlatformAutomation>,
}

impl AppState {
    /// Build the state: load the store, reconcile schedules, start the
    /// scheduler with execution wiring.
    pub fn new(
        store: JsonStore,
        automation: Arc<dyn PlatformAutomation>,
        app: AppHandle,
    ) -> AppResult<Arc<Self>> {
        let mut doc = store.load()?;

        // Reconcile persisted schedules against wall-clock reality on every
        // startup (restart recovery, spec §19). Overdue tasks follow their
        // misfire policy; `next_run_at` is recomputed for one-shots that
        // were missed and cancelled.
        let now = SystemClock.now();
        let report = crate::scheduler::reconcile(&mut doc.tasks, now);
        if !report.fire.is_empty() || !report.skipped.is_empty() {
            store.save(&doc)?;
        }

        let state = Arc::new(Self {
            doc: Mutex::new(doc),
            store,
            scheduler: Scheduler::new(
                Arc::new(SystemClock),
                // on_fire: hand execution to a dedicated thread so the
                // scheduler loop never blocks on keyboard sequences.
                {
                    let app = app.clone();
                    Arc::new(move |task| Self::fire_task(app.clone(), task))
                },
                // on_update: persist task state changes and notify the UI.
                {
                    let app = app.clone();
                    Arc::new(move |tasks| {
                        if let Err(e) = Self::persist_tasks(&app, tasks) {
                            eprintln!("persist tasks failed: {e}");
                        }
                        let _ = app.emit(events::SCHEDULER_UPDATED, tasks);
                    })
                },
            ),
            automation,
        });
        Ok(state)
    }

    /// Execute one due task on its own thread. Fire-and-forget from the
    /// scheduler's perspective; failures land in history + events.
    fn fire_task(app: AppHandle, mut task: ScheduledTask) {
        std::thread::Builder::new()
            .name(format!("exec-{}", task.name))
            .spawn(move || {
                let _ = app.emit(events::TASK_STARTED, &task.id);
                let scheduled_at = task.next_run_at.unwrap_or_else(chrono::Utc::now);
                let (record, updated_task) = {
                    let state: tauri::State<AppState> = app.state();
                    let settings = state.doc.lock().expect("doc mutex").settings.clone();
                    let outcome = executor::execute_task(
                        state.automation.as_ref(),
                        &mut task,
                        &settings,
                        &|message| {
                            let _ = app.emit(events::EXECUTION_NOTIFY, message);
                        },
                    );
                    let description = task
                        .target
                        .last_hwnd
                        .map(|_| format!("{} (cached)", task.name))
                        .unwrap_or_else(|| task.name.clone());
                    let record = executor::history_record(
                        &task,
                        scheduled_at,
                        chrono::Utc::now(),
                        &description,
                        outcome,
                    );
                    (record, task)
                };

                // Record history (bounded) and persist final task state.
                let is_success = matches!(record.outcome, ExecutionOutcome::Success);
                {
                    let state: tauri::State<AppState> = app.state();
                    let mut doc = state.doc.lock().expect("doc mutex");
                    doc.history.push(record.clone());
                    let limit = doc.settings.history_limit;
                    bound_history(&mut doc.history, limit);
                    if let Some(t) = doc.tasks.iter_mut().find(|t| t.id == updated_task.id) {
                        *t = updated_task.clone();
                    }
                    if let Err(e) = state.store.save(&doc) {
                        eprintln!("save after execution failed: {e}");
                    }
                }

                let _ = if is_success {
                    app.emit(events::TASK_COMPLETED, &record)
                } else {
                    app.emit(events::TASK_FAILED, &record)
                };
            })
            .expect("spawn execution thread");
    }

    /// Persist the given authoritative task list (scheduler callback).
    fn persist_tasks(app: &AppHandle, tasks: &[ScheduledTask]) -> AppResult<()> {
        let state: tauri::State<AppState> = app.state();
        let mut doc = state.doc.lock().expect("doc mutex");
        doc.tasks = tasks.to_vec();
        state.store.save(&doc)
    }

    /// Push the current task list into the scheduler.
    pub fn sync_scheduler(&self) {
        let tasks = self.doc.lock().expect("doc mutex").tasks.clone();
        self.scheduler.replace_tasks(tasks);
    }

    /// Emit a state-change event for a task.
    pub fn emit_task_event(&self, app: &AppHandle, event: &str, task: &ScheduledTask) {
        let _ = app.emit(event, task);
    }
}
