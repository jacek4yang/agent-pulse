//! macOS Accessibility automation through System Events. Each supported target
//! must have exactly one accessible window; multi-window apps fail as ambiguous.
use super::process::run;
use crate::error::{AppError, AppResult};
use crate::executor::PlatformAutomation;
use crate::model::{Key, WindowCandidate};

pub struct MacAutomation;
const SCRIPT: &str = include_str!("macos.js");
fn invoke(op: &str, pid: u64, value: serde_json::Value) -> AppResult<String> {
    let request = serde_json::json!({"op":op,"pid":pid,"value":value}).to_string();
    let result = run(
        "/usr/bin/osascript",
        &["-l", "JavaScript", "-e", SCRIPT, &request],
    )?;
    if result == "LOST_FOCUS" {
        return Err(AppError::TargetLostFocus);
    }
    if result == "AMBIGUOUS" {
        return Err(AppError::TargetAmbiguous);
    }
    if result == "MODIFIER_HELD" {
        return Err(AppError::ModifierKeyHeld);
    }
    Ok(result)
}
fn key_code(key: &Key) -> AppResult<u16> {
    Ok(match key {
        Key::Enter => 36,
        Key::Tab => 48,
        Key::Space => 49,
        Key::Backspace => 51,
        Key::Escape => 53,
        Key::Delete => 117,
        Key::Home => 115,
        Key::End => 119,
        Key::PageUp => 116,
        Key::PageDown => 121,
        Key::Left => 123,
        Key::Right => 124,
        Key::Down => 125,
        Key::Up => 126,
        Key::Function(n) if (1..=12).contains(n) => {
            [122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111][usize::from(*n - 1)]
        }
        _ => {
            return Err(AppError::InvalidAction(
                "key has no macOS hardware code".into(),
            ));
        }
    })
}
fn key_value(key: &Key, modifiers: &[&str]) -> AppResult<serde_json::Value> {
    match key {
        Key::Letter(c) => Ok(serde_json::json!({"text":c.to_string(),"modifiers":modifiers})),
        Key::Digit(d) => Ok(serde_json::json!({"text":d.to_string(),"modifiers":modifiers})),
        _ => Ok(serde_json::json!({"code":key_code(key)?,"modifiers":modifiers})),
    }
}
impl PlatformAutomation for MacAutomation {
    fn check_available(&self) -> AppResult<()> {
        invoke("check", 0, serde_json::Value::Null)?;
        Ok(())
    }
    fn enumerate(&self) -> Vec<WindowCandidate> {
        invoke("list", 0, serde_json::Value::Null)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn verify_cached(&self, hwnd: u64) -> Option<WindowCandidate> {
        self.enumerate().into_iter().find(|w| w.hwnd == hwnd)
    }
    fn restore(&self, _hwnd: u64) {} // Activation unminimizes the sole window.
    fn activate(&self, hwnd: u64) -> AppResult<()> {
        invoke("activate", hwnd, serde_json::Value::Null)?;
        if self.is_foreground(hwnd) {
            Ok(())
        } else {
            Err(AppError::FailedToActivateTarget)
        }
    }
    fn is_foreground(&self, hwnd: u64) -> bool {
        invoke("foreground", hwnd, serde_json::Value::Null).is_ok_and(|s| s == "true")
    }
    fn type_text(&self, hwnd: u64, text: &str) -> AppResult<()> {
        // Keep helper calls short; the script checks focus before each scalar.
        for chunk in text.chars().collect::<Vec<_>>().chunks(32) {
            invoke(
                "text",
                hwnd,
                serde_json::json!(chunk.iter().collect::<String>()),
            )?;
        }
        Ok(())
    }
    fn press_key(&self, hwnd: u64, key: Key, count: u32, interval_ms: u64) -> AppResult<()> {
        for i in 0..count {
            invoke("key", hwnd, key_value(&key, &[])?)?;
            if i + 1 < count {
                std::thread::sleep(std::time::Duration::from_millis(interval_ms));
            }
        }
        Ok(())
    }
    fn press_combination(&self, hwnd: u64, keys: &[Key]) -> AppResult<()> {
        let modifiers: Vec<&str> = keys
            .iter()
            .filter_map(|k| match k {
                Key::Ctrl => Some("control down"),
                Key::Shift => Some("shift down"),
                Key::Alt => Some("option down"),
                _ => None,
            })
            .collect();
        let others: Vec<&Key> = keys.iter().filter(|k| k.as_modifier().is_none()).collect();
        // System Events supports modifiers + one ordinary key, not held chords.
        if others.len() != 1 {
            return Err(AppError::InvalidAction(
                "macOS combinations require exactly one non-modifier key".into(),
            ));
        }
        invoke("key", hwnd, key_value(others[0], &modifiers)?)?;
        Ok(())
    }
}
