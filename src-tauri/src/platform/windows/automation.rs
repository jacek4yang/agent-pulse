//! The production [`PlatformAutomation`] implementation backed by Win32.

use crate::error::AppResult;
use crate::model::{Key, WindowCandidate};

use super::enumeration::{enumerate_visible_windows, verify_cached_window};
use super::focus::{activate_window, foreground_window, is_foreground, restore_window};
use super::input::{press_combination, press_key, type_unicode};
use crate::executor::PlatformAutomation;

/// Real desktop automation. Input injection goes to the system keyboard
/// buffer and lands in whatever window has focus — which is exactly why the
/// executor verifies the foreground window before every input step.
pub struct WindowsAutomation;

impl PlatformAutomation for WindowsAutomation {
    fn enumerate(&self) -> Vec<WindowCandidate> {
        enumerate_visible_windows()
    }

    fn verify_cached(&self, raw_hwnd: u64) -> Option<WindowCandidate> {
        verify_cached_window(raw_hwnd)
    }

    fn restore(&self, raw_hwnd: u64) {
        restore_window(raw_hwnd);
    }

    fn activate(&self, raw_hwnd: u64) -> AppResult<()> {
        activate_window(raw_hwnd)
    }

    fn is_foreground(&self, raw_hwnd: u64) -> bool {
        is_foreground(raw_hwnd)
    }

    fn type_text(&self, text: &str) -> AppResult<()> {
        type_unicode(text)
    }

    fn press_key(&self, key: Key, count: u32, interval_ms: u64) -> AppResult<()> {
        press_key(&key, count, interval_ms)
    }

    fn press_combination(&self, keys: &[Key]) -> AppResult<()> {
        press_combination(keys)
    }
}

/// The current foreground window (raw), for diagnostics.
#[allow(dead_code)]
pub fn current_foreground() -> Option<u64> {
    foreground_window()
}
