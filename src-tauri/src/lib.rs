//! Agent Pulse — application core library.

pub mod error;
pub mod model;
pub mod platform;
pub mod scheduler;
pub mod store;

pub use error::{AppError, AppResult};
pub use model::Settings;

/// Minimal placeholder command; the real command surface lands in the IPC PR.
#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "Agent Pulse",
        "version": env!("CARGO_PKG_VERSION"),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                use tauri::{Emitter, Manager};
                let window = app.get_webview_window("main");
                if let Some(w) = window {
                    let _ = w.emit("app-ready", true);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
