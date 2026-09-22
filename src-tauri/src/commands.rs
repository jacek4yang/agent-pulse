//! Typed Tauri command surface (spec §43). Rust is the source of truth.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::app_state::{AppState, events};
use crate::error::{AppError, AppResult};
use crate::model::{
    Action, HistoryRecord, ScheduledTask, Settings, TitleMatchMode, WindowCandidate, WindowTarget,
};
use crate::platform::matching::resolve_target;
use crate::store::bound_history;

#[tauri::command]
pub fn get_startup_error(state: State<crate::StartupStatus>) -> Option<String> {
    state.inner().0.clone()
}

#[tauri::command]
pub fn list_tasks(state: State<Arc<AppState>>) -> AppResult<Vec<ScheduledTask>> {
    Ok(state.lock_doc()?.tasks.clone())
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
    state.update_doc(|doc| {
        doc.tasks.push(task.clone());
        Ok(())
    })?;
    state.emit_task_event(&app, events::TASK_CREATED, &task);
    Ok(task)
}

#[tauri::command]
pub fn update_task(
    app: AppHandle,
    state: State<Arc<AppState>>,
    task: ScheduledTask,
) -> AppResult<ScheduledTask> {
    task.validate()?;
    let task = state.update_doc(|doc| {
        let existing = doc
            .tasks
            .iter_mut()
            .find(|t| t.id == task.id)
            .ok_or(AppError::TaskNotFound(task.id))?;
        existing.apply_edit(task.clone(), chrono::Utc::now())?;
        Ok(existing.clone())
    })?;
    state.emit_task_event(&app, events::TASK_UPDATED, &task);
    Ok(task)
}

#[tauri::command]
pub fn delete_task(app: AppHandle, state: State<Arc<AppState>>, task_id: Uuid) -> AppResult<()> {
    state.update_doc(|doc| {
        let before = doc.tasks.len();
        doc.tasks.retain(|t| t.id != task_id);
        if doc.tasks.len() == before {
            return Err(AppError::TaskNotFound(task_id));
        }
        Ok(())
    })?;
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
    let task = state.update_doc(|doc| {
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
        task.updated_at = chrono::Utc::now();
        Ok(task.clone())
    })?;
    state.emit_task_event(&app, events::TASK_UPDATED, &task);
    Ok(task)
}

/// Blocking platform operations run off the UI thread.
#[tauri::command]
pub async fn run_task_now(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    task_id: Uuid,
) -> AppResult<HistoryRecord> {
    let state = Arc::clone(state.inner());
    let task = state
        .lock_doc()?
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .cloned()
        .ok_or(AppError::TaskNotFound(task_id))?;
    tauri::async_runtime::spawn_blocking(move || state.execute_and_record(&app, task))
        .await
        .map_err(|e| AppError::StateUnavailable(e.to_string()))?
}

#[tauri::command]
pub async fn list_windows(state: State<'_, Arc<AppState>>) -> AppResult<Vec<WindowCandidate>> {
    let platform = Arc::clone(&state.automation);
    tauri::async_runtime::spawn_blocking(move || {
        platform.check_available()?;
        Ok(platform.enumerate())
    })
    .await
    .map_err(|e| AppError::StateUnavailable(e.to_string()))?
}

/// Validate a window target: resolve against live windows WITHOUT touching
/// the window. Returns the resolved candidate; ambiguity/not-found surface
/// as structured errors.
#[tauri::command]
pub async fn test_window_target(
    state: State<'_, Arc<AppState>>,
    target: WindowTarget,
) -> AppResult<WindowCandidate> {
    let platform = Arc::clone(&state.automation);
    tauri::async_runtime::spawn_blocking(move || {
        platform.check_available()?;
        resolve_target(&platform.enumerate(), &target)
    })
    .await
    .map_err(|e| AppError::StateUnavailable(e.to_string()))?
}

#[tauri::command]
pub fn get_history(state: State<Arc<AppState>>) -> AppResult<Vec<HistoryRecord>> {
    let doc = state.lock_doc()?;
    Ok(doc.history.iter().rev().cloned().collect())
}

#[tauri::command]
pub fn clear_history(state: State<Arc<AppState>>) -> AppResult<()> {
    state.update_doc(|doc| {
        doc.history.clear();
        Ok(())
    })
}

#[tauri::command]
pub fn get_settings(state: State<Arc<AppState>>) -> AppResult<Settings> {
    Ok(state.lock_doc()?.settings.clone())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<Arc<AppState>>,
    settings: Settings,
) -> AppResult<()> {
    apply_autostart(&app, settings.start_with_windows)?;
    state.update_doc(|doc| {
        doc.settings = settings.clone();
        bound_history(&mut doc.history, settings.history_limit);
        Ok(())
    })
}

/// Reflect the start-with-Windows preference via the autostart plugin
/// (unelevated, per-user — spec §38).
fn apply_autostart(app: &AppHandle, enabled: bool) -> AppResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    let current = autostart
        .is_enabled()
        .map_err(|e| AppError::PersistenceFailure(e.to_string()))?;
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
