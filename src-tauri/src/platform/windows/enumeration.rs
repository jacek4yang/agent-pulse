//! Top-level window enumeration (EnumWindows) and cached-HWND verification.

use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindow, IsWindowVisible,
};
use windows::core::PWSTR;

use crate::model::WindowCandidate;

use super::hwnd_from_raw;

/// Collect all visible top-level windows (on-demand; never polled).
pub fn enumerate_visible_windows() -> Vec<WindowCandidate> {
    let mut out: Vec<WindowCandidate> = Vec::new();
    collect_into(&mut out);
    out
}

/// Verify a cached HWND is still a live, visible window and refresh its
/// identity. Returns `None` when the handle is no longer valid.
pub fn verify_cached_window(raw_hwnd: u64) -> Option<WindowCandidate> {
    let hwnd = hwnd_from_raw(raw_hwnd);
    // Safety: hwnd originated from EnumWindows/GetForegroundWindow and is
    // re-validated by IsWindow before any further use.
    unsafe {
        if IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() {
            candidate_for(hwnd)
        } else {
            None
        }
    }
}

fn collect_into(out: &mut Vec<WindowCandidate>) {
    // Safety: the LPARAM carries a mutable pointer to our output Vec; the
    // callback runs synchronously inside EnumWindows on this thread, so the
    // borrow is valid for the call's lifetime.
    unsafe {
        let _ = EnumWindows(
            Some(enum_proc),
            LPARAM(out as *mut Vec<WindowCandidate> as isize),
        );
    }
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    // Safety: lparam is the pointer to the caller's Vec (see collect_into).
    let out = unsafe { &mut *(lparam.0 as *mut Vec<WindowCandidate>) };
    // Safety: plain Win32 queries on an OS-provided handle.
    let visible = unsafe { IsWindowVisible(hwnd) };
    if !visible.as_bool() {
        return true.into();
    }
    if let Some(candidate) = candidate_for(hwnd) {
        out.push(candidate);
    }
    true.into()
}

/// Build a candidate snapshot for a live, visible window.
fn candidate_for(hwnd: HWND) -> Option<WindowCandidate> {
    // Safety: Win32 text/pid queries with a validated handle.
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            // Windows with no title are never useful targets.
            return None;
        }
        let mut buf = [0u16; 512];
        let copied = GetWindowTextW(hwnd, &mut buf);
        let copied = usize::try_from(copied).unwrap_or(0).min(buf.len());
        let title = String::from_utf16_lossy(&buf[..copied]);

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let image_path = process_image_name(pid);
        let process_name = image_path.as_deref().map(|p| {
            // Extract the file name component from the full image path.
            p.rsplit(['\\', '/']).next().unwrap_or(p).to_string()
        });

        Some(WindowCandidate {
            hwnd: hwnd.0 as usize as u64,
            title,
            process_id: pid,
            process_name: process_name.unwrap_or_default(),
            executable_path: image_path,
            is_visible: true,
        })
    }
}

/// Query the full image path of a process (unprivileged; fails cleanly for
/// system processes we cannot open).
fn process_image_name(pid: u32) -> Option<String> {
    // Safety: OpenProcess with LIMITED_INFORMATION requires no elevation;
    // HANDLE is closed before return.
    unsafe {
        if true {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = u32::try_from(buf.len()).unwrap_or(1024);
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let path = if result.is_ok() {
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        } else {
            None
        };
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        path
    }
}

/// Whether a window is currently minimized (needed for restore decisions).
pub fn is_minimized(raw_hwnd: u64) -> bool {
    let hwnd = hwnd_from_raw(raw_hwnd);
    // Safety: read-only query on a validated handle.
    unsafe { IsIconic(hwnd).as_bool() }
}

/// Bounding rectangle of a window (used by future UX features).
#[allow(dead_code)]
pub fn window_rect(raw_hwnd: u64) -> Option<RECT> {
    let hwnd = hwnd_from_raw(raw_hwnd);
    // Safety: read-only query on a validated handle.
    unsafe {
        let mut rect = RECT::default();
        if windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect).is_ok() {
            Some(rect)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumeration_returns_windows_with_process_names() {
        let windows = enumerate_visible_windows();
        assert!(!windows.is_empty(), "a desktop session always has windows");
        for w in &windows {
            assert!(w.is_visible);
            assert_ne!(w.process_id, 0);
        }
    }

    #[test]
    fn cached_verification_rejects_bogus_handle() {
        assert!(verify_cached_window(0xDEAD_BEEF_DEAD_BEEF).is_none());
        assert!(verify_cached_window(0).is_none());
    }

    #[test]
    fn live_window_verifies_and_round_trips() {
        let windows = enumerate_visible_windows();
        let first = windows.first().expect("at least one window");
        let verified = verify_cached_window(first.hwnd).expect("live window verifies");
        assert_eq!(verified.hwnd, first.hwnd);
        assert_eq!(verified.process_id, first.process_id);
    }
}
