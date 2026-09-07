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
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = app.path().app_data_dir().expect("app data dir unavailable");
            let store = JsonStore::new(data_dir.join("store.json"));

            #[cfg(windows)]
            let automation: Arc<dyn PlatformAutomation> =
                Arc::new(platform::windows::WindowsAutomation);
            #[cfg(not(windows))]
            let automation: Arc<dyn PlatformAutomation> = Arc::new(UnsupportedAutomation);

            let state = AppState::new(store, automation, handle)?;
            app.manage(std::sync::Arc::clone(&state));
            state.scheduler.clone().start();
            state.sync_scheduler();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Placeholder for non-Windows compilation.
#[cfg(not(windows))]
struct UnsupportedAutomation;

#[cfg(not(windows))]
impl PlatformAutomation for UnsupportedAutomation {
    fn enumerate(&self) -> Vec<model::WindowCandidate> {
        Vec::new()
    }
    fn verify_cached(&self, _: u64) -> Option<model::WindowCandidate> {
        None
    }
    fn restore(&self, _: u64) {}
    fn activate(&self, _: u64) -> AppResult<()> {
        Err(AppError::FailedToActivateTarget)
    }
    fn is_foreground(&self, _: u64) -> bool {
        false
    }
    fn type_text(&self, _: &str) -> AppResult<()> {
        Err(AppError::InputInjectionFailed)
    }
    fn press_key(&self, _: model::Key, _: u32, _: u64) -> AppResult<()> {
        Err(AppError::InputInjectionFailed)
    }
    fn press_combination(&self, _: &[model::Key]) -> AppResult<()> {
        Err(AppError::InputInjectionFailed)
    }
}
