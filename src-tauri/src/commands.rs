//! Typed Tauri command surface (spec §43). Rust is the source of truth.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::app_state::{AppState, events};
use crate::error::{AppError, AppResult};
use crate::executor::run_task_once;
use crate::model::{
    Action, ExecutionOutcome, HistoryRecord, ScheduledTask, Settings, TitleMatchMode,
    WindowCandidate, WindowTarget,
};
use crate::platform::matching::resolve_target;
use crate::store::bound_history;

#[tauri::command]
pub fn list_tasks(state: State<Arc<AppState>>) -> Vec<ScheduledTask> {
    state.doc.lock().expect("doc mutex").tasks.clone()
}

#[tauri::command]
pub fn create_task(
    app: AppHandle,
    state: State<Arc<AppState>>,
    name: String,
    target: WindowTarget,
    schedule: crate::model::Schedule,
    actions: Vec<Action>,
    misfire_policy: crate::model::MisfirePolicy,
) -> AppResult<ScheduledTask> {
    if name.trim().is_empty() {
        return Err(AppError::InvalidSchedule(
            "task name must not be empty".into(),
        ));
    }
    let now = chrono::Utc::now();
    let task = ScheduledTask::new(
        name.trim().to_string(),
        target,
        schedule,
        actions,
        misfire_policy,
        now,
    )?;
    {
        let mut doc = state.doc.lock().expect("doc mutex");
        doc.tasks.push(task.clone());
        state.store.save(&doc)?;
    }
    state.sync_scheduler();
    state.emit_task_event(&app, events::TASK_CREATED, &task);
    Ok(task)
}

#[tauri::command]
pub fn update_task(
    app: AppHandle,
    state: State<Arc<AppState>>,
    task: ScheduledTask,
) -> AppResult<ScheduledTask> {
    let now = chrono::Utc::now();
    let mut task = task;
    task.updated_at = now;
    // Re-arm relative schedules from now on edit.
    if matches!(task.schedule, crate::model::Schedule::After { .. }) || task.next_run_at.is_none() {
        task.rearm(now)?;
    }
    {
        let mut doc = state.doc.lock().expect("doc mutex");
        let existing = doc
            .tasks
            .iter_mut()
            .find(|t| t.id == task.id)
            .ok_or(AppError::TaskNotFound(task.id))?;
        *existing = task.clone();
        state.store.save(&doc)?;
    }
    state.sync_scheduler();
    state.emit_task_event(&app, events::TASK_UPDATED, &task);
    Ok(task)
}

#[tauri::command]
pub fn delete_task(app: AppHandle, state: State<Arc<AppState>>, task_id: Uuid) -> AppResult<()> {
    {
        let mut doc = state.doc.lock().expect("doc mutex");
        let before = doc.tasks.len();
        doc.tasks.retain(|t| t.id != task_id);
        if doc.tasks.len() == before {
            return Err(AppError::TaskNotFound(task_id));
        }
        state.store.save(&doc)?;
    }
    state.sync_scheduler();
    let _ = app.emit(events::SCHEDULER_UPDATED, ());
    Ok(())
}

#[tauri::command]
pub fn set_task_enabled(
    app: AppHandle,
    state: State<Arc<AppState>>,
    task_id: Uuid,
    enabled: bool,
) -> AppResult<ScheduledTask> {
    let task = {
        let mut doc = state.doc.lock().expect("doc mutex");
        let task = doc
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or(AppError::TaskNotFound(task_id))?;
        task.enabled = enabled;
        if enabled && task.next_run_at.is_none() {
            task.rearm(chrono::Utc::now())?;
        }
        if !enabled {
            task.next_run_at = None;
        }
        task.clone()
    };
    state.store.update(|doc| {
        if let Some(t) = doc.tasks.iter_mut().find(|t| t.id == task_id) {
            t.enabled = task.enabled;
            t.next_run_at = task.next_run_at;
        }
        Ok(())
    })?;
    state.sync_scheduler();
    state.emit_task_event(&app, events::TASK_UPDATED, &task);
    Ok(task)
}

/// Run a task immediately, bypassing the schedule. Synchronous: the command
/// returns once the sequence finished (or failed). Uses the same global
/// execution lock as scheduled runs.
#[tauri::command]
pub fn run_task_now(
    app: AppHandle,
    state: State<Arc<AppState>>,
    task_id: Uuid,
) -> AppResult<HistoryRecord> {
    let settings = state.doc.lock().expect("doc mutex").settings.clone();
    let mut task = {
        let doc = state.doc.lock().expect("doc mutex");
        doc.tasks
            .iter()
            .find(|t| t.id == task_id)
            .cloned()
            .ok_or(AppError::TaskNotFound(task_id))?
    };
    let scheduled_at = task.next_run_at.unwrap_or_else(chrono::Utc::now);
    let record = run_task_once(
        state.automation.as_ref(),
        &mut task,
        &settings,
        scheduled_at,
        &|_| {},
    )?;

    // Persist updated task state + history.
    {
        let mut doc = state.doc.lock().expect("doc mutex");
        if let Some(t) = doc.tasks.iter_mut().find(|t| t.id == task_id) {
            t.last_run_at = task.last_run_at;
            t.target.last_hwnd = task.target.last_hwnd;
        }
        doc.history.push(record.clone());
        let limit = doc.settings.history_limit;
        bound_history(&mut doc.history, limit);
        state.store.save(&doc)?;
    }
    state.sync_scheduler();

    let success = matches!(record.outcome, ExecutionOutcome::Success);
    let _ = if success {
        app.emit(events::TASK_COMPLETED, &record)
    } else {
        app.emit(events::TASK_FAILED, &record)
    };
    Ok(record)
}

#[tauri::command]
pub fn list_windows() -> Vec<WindowCandidate> {
    crate::platform::windows::enumerate_visible_windows()
}

/// Validate a window target: resolve against live windows WITHOUT touching
/// the window. Returns the resolved candidate; ambiguity/not-found surface
/// as structured errors.
#[tauri::command]
pub fn test_window_target(
    state: State<Arc<AppState>>,
    target: WindowTarget,
) -> AppResult<WindowCandidate> {
    let _ = state; // reserved for future logging hooks
    let candidates = crate::platform::windows::enumerate_visible_windows();
    resolve_target(&candidates, &target)
}

#[tauri::command]
pub fn get_history(state: State<Arc<AppState>>) -> Vec<HistoryRecord> {
    let doc = state.doc.lock().expect("doc mutex");
    doc.history.iter().rev().cloned().collect()
}

#[tauri::command]
pub fn clear_history(state: State<Arc<AppState>>) -> AppResult<()> {
    state.store.update(|doc| {
        doc.history.clear();
        Ok(())
    })
}

#[tauri::command]
pub fn get_settings(state: State<Arc<AppState>>) -> Settings {
    state.doc.lock().expect("doc mutex").settings.clone()
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<Arc<AppState>>,
    settings: Settings,
) -> AppResult<()> {
    state.store.update(|doc| {
        doc.settings = settings.clone();
        Ok(())
    })?;
    apply_autostart(&app, settings.start_with_windows)
}

/// Reflect the start-with-Windows preference via the autostart plugin
/// (unelevated, per-user — spec §38).
fn apply_autostart(app: &AppHandle, enabled: bool) -> AppResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    let current = autostart.is_enabled().unwrap_or(false);
    if enabled && !current {
        autostart.enable().map_err(|e| {
            AppError::PersistenceFailure(format!("failed to enable autostart: {e}"))
        })?;
    } else if !enabled && current {
        autostart.disable().map_err(|e| {
            AppError::PersistenceFailure(format!("failed to disable autostart: {e}"))
        })?;
    }
    Ok(())
}

/// Build a quick task from the fast-creation flow (spec §33). Fully
/// flexible: any schedule (After / At / Every, second precision) and any
/// ordered action list (a preset expansion or a fully custom flow). The
/// name is auto-derived from the schedule when omitted.
#[tauri::command]
pub fn create_quick_task(
    app: AppHandle,
    state: State<Arc<AppState>>,
    name: Option<String>,
    target: WindowTarget,
    schedule: crate::model::Schedule,
    actions: Vec<Action>,
) -> AppResult<ScheduledTask> {
    let name = name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| describe_schedule(&schedule));
    create_task(
        app,
        state,
        name,
        target,
        schedule,
        actions,
        crate::model::MisfirePolicy::default(),
    )
}

/// Human-readable schedule summary used as the default task name.
fn describe_schedule(schedule: &crate::model::Schedule) -> String {
    match schedule {
        crate::model::Schedule::After {
            hours,
            minutes,
            seconds,
        } => {
            format!("Automation — after {hours}h {minutes:02}m {seconds:02}s")
        }
        crate::model::Schedule::At {
            year,
            month,
            day,
            hour,
            minute,
            second,
            timezone,
        } => {
            let tz = if timezone.is_empty() {
                "local".to_string()
            } else {
                timezone.clone()
            };
            format!(
                "Automation — at {year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} ({tz})"
            )
        }
        crate::model::Schedule::Every { interval_seconds } => {
            format!("Automation — every {interval_seconds}s")
        }
    }
}

/// Title match modes exposed to the picker UI.
#[tauri::command]
pub fn default_title_match_mode() -> TitleMatchMode {
    TitleMatchMode::Contains
}
