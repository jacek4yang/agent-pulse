//! System tray: Agent Pulse keeps scheduling while hidden (D11).

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

use crate::app_state::AppState;

pub const TRAY_MENU_EVENT: &str = "tray-menu";

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let pause_all = MenuItem::with_id(app, "pause_all", "Pause All", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &separator1, &pause_all, &separator2, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Agent Pulse")
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "open" => show_main(app),
                "quit" => {
                    // Explicit quit: stop the scheduler cleanly and exit.
                    let state: tauri::State<AppState> = app.state();
                    state.scheduler.stop();
                    app.exit(0);
                }
                "pause_all" => {
                    let state: tauri::State<AppState> = app.state();
                    let paused = !state.scheduler.is_paused();
                    state.scheduler.set_paused(paused);
                    let _ = app.emit("tray-pause-changed", paused);
                }
                _ => {}
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }
    tray.build(app)?;
    Ok(())
}

/// Show and focus the main window (idempotent when already visible).
pub fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
