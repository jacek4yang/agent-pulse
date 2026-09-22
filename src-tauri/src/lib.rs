//! Agent Pulse — application core library.

pub mod app_state;
pub mod commands;
pub mod error;
pub mod executor;
pub mod model;
pub mod platform;
pub mod scheduler;
pub mod store;
pub mod tray;

use std::sync::Arc;

use tauri::Manager;

pub use error::{AppError, AppResult};
pub use model::Settings;

use crate::app_state::AppState;
use crate::executor::PlatformAutomation;
use crate::store::JsonStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub struct StartupStatus(pub Option<String>);

pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show_main(app)
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            // Keep a visible diagnostics UI when loading fails. Never replace
            // unreadable/newer user data with an empty writable store.
            let initialized = (|| -> AppResult<()> {
                let handle = app.handle().clone();
                let data_dir = app
                    .path()
                    .app_data_dir()
                    .map_err(|e| AppError::PersistenceFailure(e.to_string()))?;
                let store = JsonStore::new(data_dir.join("store.json"));
                let automation: Arc<dyn PlatformAutomation> = platform::automation();
                let state = AppState::new(store, automation, handle)?;
                app.manage(Arc::clone(&state));
                let has_tray = match tray::setup_tray(app.handle()) {
                    Ok(()) => true,
                    Err(error) => {
                        eprintln!("tray unavailable: {error}");
                        false
                    }
                };
                state.sync_scheduler()?;
                state.scheduler.start()?;
                if has_tray
                    && state.lock_doc()?.settings.start_minimized
                    && let Some(window) = app.get_webview_window("main")
                {
                    let _ = window.hide();
                }
                Ok(())
            })();
            app.manage(StartupStatus(initialized.err().map(|e| e.to_string())));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.app_handle().tray_by_id("main-tray").is_some()
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_startup_error,
            commands::list_tasks,
            commands::create_task,
            commands::update_task,
            commands::delete_task,
            commands::set_task_enabled,
            commands::run_task_now,
            commands::list_windows,
            commands::test_window_target,
            commands::get_history,
            commands::clear_history,
            commands::get_settings,
            commands::update_settings,
            commands::create_quick_task,
            commands::default_title_match_mode,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("Agent Pulse could not start: {error}");
    }
}
