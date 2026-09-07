//! Platform abstraction trait. The executor only ever talks to this trait —
//! unit tests use the mock implementation and never touch the real desktop.

use crate::error::AppResult;
use crate::model::{Key, WindowCandidate};

/// Window automation operations needed by the action executor.
///
/// Contract notes (safety-critical, see docs/DECISIONS.md D5/D6/D7):
/// - `activate` MUST only return `Ok(())` when the target has actually been
///   verified as the foreground window.
/// - Input methods (`type_text`, `press_key`, `press_combination`) inject
///   into whatever is focused; the executor therefore verifies the foreground
///   window before every meaningful input step.
pub trait PlatformAutomation: Send + Sync {
    /// All visible top-level windows.
    fn enumerate(&self) -> Vec<WindowCandidate>;
    /// Verify a cached HWND and refresh its identity, if still valid.
    fn verify_cached(&self, raw_hwnd: u64) -> Option<WindowCandidate>;
    /// Restore a minimized window.
    fn restore(&self, raw_hwnd: u64);
    /// Activate and VERIFY the target is foreground. Fails otherwise.
    fn activate(&self, raw_hwnd: u64) -> AppResult<()>;
    /// Is the given window currently the foreground window?
    fn is_foreground(&self, raw_hwnd: u64) -> bool;
    /// Type Unicode text as keyboard events. Never uses the clipboard.
    fn type_text(&self, text: &str) -> AppResult<()>;
    /// Press a key `count` times with an interval between presses.
    fn press_key(&self, key: Key, count: u32, interval_ms: u64) -> AppResult<()>;
    /// Press a key combination with press-and-hold semantics.
    fn press_combination(&self, keys: &[Key]) -> AppResult<()>;
}
