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

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod process;

pub fn automation() -> std::sync::Arc<dyn crate::executor::PlatformAutomation> {
    #[cfg(windows)]
    {
        std::sync::Arc::new(windows::WindowsAutomation)
    }
    #[cfg(target_os = "linux")]
    {
        std::sync::Arc::new(linux::LinuxAutomation)
    }
    #[cfg(target_os = "macos")]
    {
        std::sync::Arc::new(macos::MacAutomation)
    }
}
