//! Linux X11 automation. Wayland deliberately fails closed: XWayland cannot
//! verify the global foreground across native Wayland clients.
use super::process::run;
use crate::error::{AppError, AppResult};
use crate::executor::PlatformAutomation;
use crate::model::{Key, WindowCandidate};

pub struct LinuxAutomation;

fn xdo(args: &[&str]) -> AppResult<String> {
    run("xdotool", args)
}
fn key_name(key: &Key) -> String {
    match key {
        Key::Enter => "Return".into(),
        Key::Escape => "Escape".into(),
        Key::Tab => "Tab".into(),
        Key::Space => "space".into(),
        Key::Backspace => "BackSpace".into(),
        Key::Delete => "Delete".into(),
        Key::Home => "Home".into(),
        Key::End => "End".into(),
        Key::PageUp => "Prior".into(),
        Key::PageDown => "Next".into(),
        Key::Up => "Up".into(),
        Key::Down => "Down".into(),
        Key::Left => "Left".into(),
        Key::Right => "Right".into(),
        Key::Ctrl => "ctrl".into(),
        Key::Shift => "shift".into(),
        Key::Alt => "alt".into(),
        Key::Letter(c) => c.to_string(),
        Key::Digit(d) => d.to_string(),
        Key::Function(n) => format!("F{n}"),
    }
}

fn check_modifiers() -> AppResult<()> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt;
    let fail = |e: &dyn std::fmt::Display| {
        AppError::AutomationUnavailable(format!("X11 keyboard state: {e}"))
    };
    let (conn, _) = x11rb::connect(None).map_err(|e| fail(&e))?;
    let pressed = conn
        .query_keymap()
        .map_err(|e| fail(&e))?
        .reply()
        .map_err(|e| fail(&e))?;
    let setup = conn.setup();
    let map = conn
        .get_keyboard_mapping(setup.min_keycode, setup.max_keycode - setup.min_keycode + 1)
        .map_err(|e| fail(&e))?
        .reply()
        .map_err(|e| fail(&e))?;
    if map.keysyms_per_keycode == 0 {
        return Err(AppError::AutomationUnavailable(
            "empty X11 keyboard mapping".into(),
        ));
    }
    for (i, symbols) in map
        .keysyms
        .chunks(usize::from(map.keysyms_per_keycode))
        .enumerate()
    {
        let code = i + usize::from(setup.min_keycode);
        if pressed.keys[code / 8] & (1 << (code % 8)) != 0
            && symbols
                .iter()
                .any(|s| matches!(*s, 0xffe1..=0xffe4 | 0xffe7..=0xffee))
        {
            return Err(AppError::ModifierKeyHeld);
        }
    }
    Ok(())
}

impl LinuxAutomation {
    fn guard(&self, hwnd: u64) -> AppResult<()> {
        if !self.is_foreground(hwnd) {
            return Err(AppError::TargetLostFocus);
        }
        check_modifiers()
    }
}

impl PlatformAutomation for LinuxAutomation {
    fn check_available(&self) -> AppResult<()> {
        if std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
        {
            return Err(AppError::AutomationUnavailable(
                "Wayland is not supported; log into an X11/Xorg desktop session".into(),
            ));
        }
        if std::env::var_os("DISPLAY").is_none() {
            return Err(AppError::AutomationUnavailable(
                "an active X11 desktop is required".into(),
            ));
        }
        xdo(&["version"])?;
        Ok(())
    }
    fn enumerate(&self) -> Vec<WindowCandidate> {
        // EWMH client list includes minimized windows and excludes menus/tooltips.
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
        let ids = (|| -> Option<Vec<u32>> {
            let (conn, screen) = x11rb::connect(None).ok()?;
            let atom = conn
                .intern_atom(false, b"_NET_CLIENT_LIST")
                .ok()?
                .reply()
                .ok()?
                .atom;
            let reply = conn
                .get_property(
                    false,
                    conn.setup().roots[screen].root,
                    atom,
                    AtomEnum::WINDOW,
                    0,
                    4096,
                )
                .ok()?
                .reply()
                .ok()?;
            Some(reply.value32()?.collect())
        })()
        .unwrap_or_default();
        let ids = ids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        ids.lines()
            .filter_map(|id| {
                let hwnd = id.parse().ok()?;
                let title = xdo(&["getwindowname", id]).ok()?;
                let process_id: u32 = xdo(&["getwindowpid", id]).ok()?.parse().ok()?;
                let path = std::fs::read_link(format!("/proc/{process_id}/exe")).ok()?;
                Some(WindowCandidate {
                    hwnd,
                    title,
                    process_id,
                    process_name: path.file_name()?.to_string_lossy().into_owned(),
                    executable_path: Some(path.to_string_lossy().into_owned()),
                    is_visible: true,
                })
            })
            .collect()
    }
    fn verify_cached(&self, hwnd: u64) -> Option<WindowCandidate> {
        self.enumerate().into_iter().find(|w| w.hwnd == hwnd)
    }
    fn restore(&self, _hwnd: u64) {} // EWMH activation restores minimized windows.
    fn activate(&self, hwnd: u64) -> AppResult<()> {
        xdo(&["windowactivate", "--sync", &hwnd.to_string()])?;
        if self.is_foreground(hwnd) {
            Ok(())
        } else {
            Err(AppError::FailedToActivateTarget)
        }
    }
    fn is_foreground(&self, hwnd: u64) -> bool {
        xdo(&["getactivewindow"])
            .ok()
            .and_then(|id| id.parse::<u64>().ok())
            == Some(hwnd)
    }
    fn type_text(&self, hwnd: u64, text: &str) -> AppResult<()> {
        for c in text.chars() {
            self.guard(hwnd)?;
            xdo(&["type", "--delay", "0", "--", &c.to_string()])?;
        }
        Ok(())
    }
    fn press_key(&self, hwnd: u64, key: Key, count: u32, interval_ms: u64) -> AppResult<()> {
        for i in 0..count {
            self.guard(hwnd)?;
            xdo(&["key", &key_name(&key)])?;
            if i + 1 < count {
                std::thread::sleep(std::time::Duration::from_millis(interval_ms));
            }
        }
        Ok(())
    }
    fn press_combination(&self, hwnd: u64, keys: &[Key]) -> AppResult<()> {
        self.guard(hwnd)?;
        xdo(&[
            "key",
            &keys.iter().map(key_name).collect::<Vec<_>>().join("+"),
        ])?;
        Ok(())
    }
}
