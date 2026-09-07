//! Safe foreground activation (spec §29). Execution order is mandatory:
//! resolve → restore → activate → verify → inject. Activation failure or a
//! foreground mismatch aborts BEFORE any keyboard input is sent.

use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsIconic, IsWindow, IsWindowVisible, SW_RESTORE, SetForegroundWindow,
    ShowWindow,
};

use crate::error::{AppError, AppResult};

use super::hwnd_from_raw;

/// Raw HWND of the current foreground window.
pub fn foreground_window() -> Option<u64> {
    // Safety: read-only query returning an OS handle.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd.0 as usize as u64)
        }
    }
}

/// Is the given raw HWND the current foreground window?
pub fn is_foreground(raw_hwnd: u64) -> bool {
    foreground_window().is_some_and(|fg| fg == raw_hwnd)
}

/// Restore the window if it is minimized. Cheap no-op otherwise.
pub fn restore_window(raw_hwnd: u64) {
    let hwnd = hwnd_from_raw(raw_hwnd);
    // Safety: ShowWindow with a validated handle; SW_RESTORE is a documented,
    // unprivileged operation.
    unsafe {
        if IsWindow(Some(hwnd)).as_bool() && IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
    }
}

/// Bring the target to the foreground and VERIFY. Retries briefly because
/// SetForegroundWindow is asynchronous under Windows' focus rules. Returns
/// `FailedToActivateTarget` if verification does not pass — the caller must
/// then send no input.
pub fn activate_window(raw_hwnd: u64) -> AppResult<()> {
    let hwnd = hwnd_from_raw(raw_hwnd);
    // Safety: handle is validated before activation; SetForegroundWindow is
    // a documented unprivileged API. We call it only for a resolved target.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err(AppError::TargetNotFound);
        }
        if !IsWindowVisible(hwnd).as_bool() {
            return Err(AppError::TargetNotVisible);
        }

        // A short retry loop: focus changes are asynchronous and the OS may
        // refuse the first call depending on foreground-lock state.
        for attempt in 0..5 {
            let _ = SetForegroundWindow(hwnd);
            std::thread::sleep(std::time::Duration::from_millis(
                50u64.saturating_mul(attempt + 1),
            ));
            if foreground_window().is_some_and(|fg| fg == raw_hwnd) {
                return Ok(());
            }
        }
    }
    Err(AppError::FailedToActivateTarget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_of_bogus_handle_fails_without_input() {
        assert!(matches!(
            activate_window(0xDEAD_BEEF_DEAD_BEEF),
            Err(AppError::TargetNotFound)
        ));
    }

    #[test]
    fn foreground_query_does_not_crash() {
        // In a CI desktop session there is usually *some* foreground window;
        // either result is acceptable, the API must simply work.
        let _ = foreground_window();
    }
}
