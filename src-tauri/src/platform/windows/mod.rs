//! Win32 backend (spec §26). All `unsafe` FFI is confined to this module;
//! every block states its safety invariant. Documented, unprivileged APIs
//! only — the app never requires elevation (docs/DECISIONS.md D12).

pub(crate) mod automation;
pub(crate) mod enumeration;
pub(crate) mod focus;
pub(crate) mod input;

pub use automation::WindowsAutomation;
pub use enumeration::{enumerate_visible_windows, is_minimized, verify_cached_window};
pub use focus::{activate_window, foreground_window, is_foreground, restore_window};

/// Convert a raw HWND value stored in [`crate::model::WindowTarget::last_hwnd`]
/// into a typed `HWND`.
pub(crate) fn hwnd_from_raw(raw: u64) -> windows::Win32::Foundation::HWND {
    // Safety: HWND is a transparent wrapper around a pointer-sized handle
    // value previously obtained from the OS; we only ever pass it back to
    // Win32 APIs that validate it (IsWindow etc.).
    windows::Win32::Foundation::HWND(raw as *mut core::ffi::c_void)
}
