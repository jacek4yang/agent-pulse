//! Platform abstraction. Core logic only ever talks to these functions /
//! the `PlatformAutomation` trait, never to Win32 directly.

pub mod matching;

#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::{
    activate_window, enumerate_visible_windows, foreground_window, is_foreground, is_minimized,
    restore_window, verify_cached_window,
};
