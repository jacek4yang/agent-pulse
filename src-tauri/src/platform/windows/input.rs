//! Keyboard input injection via SendInput (spec §26, §27). Unicode typing
//! uses KEYEVENTF_UNICODE events; the clipboard is never touched (D3).

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC,
    MapVirtualKeyW, SendInput, VIRTUAL_KEY,
};

use crate::error::{AppError, AppResult};
use crate::model::Key;

/// Virtual-key codes for the strongly-typed `Key` enum (spec §28).
/// Returns `None` for keys that have no fixed VK (handled elsewhere).
pub(crate) fn virtual_key(key: &Key) -> Option<u16> {
    let vk = match key {
        Key::Enter => 0x0D,     // VK_RETURN
        Key::Escape => 0x1B,    // VK_ESCAPE
        Key::Tab => 0x09,       // VK_TAB
        Key::Space => 0x20,     // VK_SPACE
        Key::Backspace => 0x08, // VK_BACK
        Key::Delete => 0x2E,    // VK_DELETE
        Key::Home => 0x24,      // VK_HOME
        Key::End => 0x23,       // VK_END
        Key::PageUp => 0x21,    // VK_PRIOR
        Key::PageDown => 0x22,  // VK_NEXT
        Key::Up => 0x26,        // VK_UP
        Key::Down => 0x28,      // VK_DOWN
        Key::Left => 0x25,      // VK_LEFT
        Key::Right => 0x27,     // VK_RIGHT
        Key::Ctrl => 0x11,      // VK_CONTROL
        Key::Shift => 0x10,     // VK_SHIFT
        Key::Alt => 0x12,       // VK_MENU
        Key::Letter(c) => {
            let upper = c.to_ascii_uppercase();
            if !upper.is_ascii_uppercase() {
                return None;
            }
            u16::from(upper as u8) // 'A'..'Z' == VK 0x41..0x5A
        }
        Key::Digit(d) => match d {
            0..=9 => u16::from(b'0' + *d), // '0'..'9' == VK 0x30..0x39
            _ => return None,
        },
        Key::Function(n) => match n {
            1..=12 => 0x6F + u16::from(*n), // VK_F1 = 0x70
            _ => return None,
        },
    };
    Some(vk)
}

/// Safety invariant for all SendInput calls below: INPUT structs are plain
/// value types built on the stack with initialized unions; SendInput copies
/// them synchronously. Failures are surfaced as `InputInjectionFailed`.
fn send(raw_hwnd: u64, inputs: &[INPUT]) -> AppResult<()> {
    if !super::focus::is_foreground(raw_hwnd) {
        return Err(AppError::TargetLostFocus);
    }
    // Safety: read-only keyboard state query. Do not turn Enter into a user-held
    // Shift/Ctrl/Alt/Windows shortcut or release keys the user is holding.
    let modifier_held = [0x10, 0x11, 0x12, 0x5B, 0x5C]
        .iter()
        .any(|vk| unsafe { GetAsyncKeyState(*vk) < 0 });
    if modifier_held {
        return Err(AppError::ModifierKeyHeld);
    }
    // Safety: inputs is a fully initialized slice of INPUT structs.
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        // A short write may have left one of OUR keys down. Release only keys
        // accepted in this batch, and only while the verified target still owns
        // focus. Never continue the sequence or replay the failed text/key.
        let releases = pending_releases(inputs, sent as usize);
        if !releases.is_empty() && super::focus::is_foreground(raw_hwnd) {
            // Safety: initialized keyboard INPUT values, same verified target;
            // these are key-up events for this call's accepted key-downs only.
            let _ = unsafe { SendInput(&releases, std::mem::size_of::<INPUT>() as i32) };
        }
        return Err(AppError::InputInjectionFailed);
    }
    Ok(())
}

fn pending_releases(inputs: &[INPUT], sent: usize) -> Vec<INPUT> {
    let mut held: Vec<KEYBDINPUT> = Vec::new();
    for input in inputs.iter().take(sent) {
        // Safety: all callers build INPUT_KEYBOARD with the ki member initialized.
        let key = unsafe { input.Anonymous.ki };
        if key.dwFlags.contains(KEYEVENTF_KEYUP) {
            held.retain(|down| down.wVk != key.wVk || down.wScan != key.wScan);
        } else {
            held.push(key);
        }
    }
    held.iter()
        .rev()
        .map(|key| keyboard_input(key.wVk.0, key.wScan, key.dwFlags | KEYEVENTF_KEYUP))
        .collect()
}

fn keyboard_input(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key_event(vk: u16, up: bool) -> INPUT {
    let mut flags = if up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    // Main Enter uses a physical scan code, never VK_PACKET / a Unicode newline.
    // Navigation keys are extended; without this bit terminals may see numpad keys.
    let scan = if vk == 0x0D {
        0x1C
    } else {
        // Safety: pure mapping of a validated virtual key using the current layout.
        unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) as u16 }
    };
    if matches!(vk, 0x21..=0x28 | 0x2E) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if vk == 0x0D {
        flags |= KEYEVENTF_SCANCODE;
    }
    keyboard_input(vk, scan, flags)
}

/// Type arbitrary Unicode text as KEYEVENTF_UNICODE down/up pairs.
/// Characters outside the BMP are sent as surrogate pairs.
pub fn type_unicode(raw_hwnd: u64, text: &str) -> AppResult<()> {
    // Bound each batch to one Unicode scalar so focus is rechecked throughout
    // long text and surrogate pairs stay together in the same SendInput call.
    for ch in text.chars() {
        let mut utf16 = [0u16; 2];
        let mut inputs = Vec::with_capacity(4);
        for unit in ch.encode_utf16(&mut utf16) {
            inputs.push(keyboard_input(0, *unit, KEYEVENTF_UNICODE));
            inputs.push(keyboard_input(
                0,
                *unit,
                KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
            ));
        }
        send(raw_hwnd, &inputs)?;
    }
    Ok(())
}

/// Press a key `count` times with an interval between presses.
pub fn press_key(raw_hwnd: u64, key: &Key, count: u32, interval_ms: u64) -> AppResult<()> {
    let Some(vk) = virtual_key(key) else {
        return Err(AppError::InputInjectionFailed);
    };
    for i in 0..count {
        send(raw_hwnd, &[key_event(vk, false), key_event(vk, true)])?;
        if i + 1 < count && interval_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        }
    }
    Ok(())
}

/// Press a combination: modifiers down first, then non-modifiers, then
/// release everything in reverse order (Ctrl+C releases C before Ctrl).
pub fn press_combination(raw_hwnd: u64, keys: &[Key]) -> AppResult<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let mut sequence: Vec<(&Key, bool)> = Vec::with_capacity(keys.len() * 2);
    let (modifiers, others): (Vec<&Key>, Vec<&Key>) =
        keys.iter().partition(|k| k.as_modifier().is_some());
    for k in &modifiers {
        sequence.push((k, false));
    }
    for k in &others {
        sequence.push((k, false));
    }
    // Release in reverse press order (Ctrl+C releases C before Ctrl).
    let released: Vec<(&Key, bool)> = sequence.iter().rev().map(|(k, _)| (*k, true)).collect();
    sequence.extend(released);

    let mut inputs = Vec::with_capacity(sequence.len());
    for (k, up) in &sequence {
        let Some(vk) = virtual_key(k) else {
            return Err(AppError::InputInjectionFailed);
        };
        inputs.push(key_event(vk, *up));
    }
    send(raw_hwnd, &inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_vk_mapping_is_complete() {
        let keys = [
            Key::Enter,
            Key::Escape,
            Key::Tab,
            Key::Space,
            Key::Backspace,
            Key::Delete,
            Key::Home,
            Key::End,
            Key::PageUp,
            Key::PageDown,
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::Ctrl,
            Key::Shift,
            Key::Alt,
        ];
        for k in &keys {
            assert!(virtual_key(k).is_some(), "{k:?} must map to a VK");
        }
        for i in 0..26u8 {
            assert_eq!(
                virtual_key(&Key::Letter((b'a' + i) as char)),
                Some(u16::from(b'A' + i))
            );
        }
        for d in 0..=9u8 {
            assert_eq!(virtual_key(&Key::Digit(d)), Some(u16::from(b'0' + d)));
        }
        for n in 1..=12u8 {
            assert_eq!(virtual_key(&Key::Function(n)), Some(0x6F + u16::from(n)));
        }
        // Out of domain: lowercase non-letters and invalid digits/functions.
        assert!(virtual_key(&Key::Letter('!')).is_none());
        assert!(virtual_key(&Key::Function(13)).is_none());
    }

    #[test]
    fn enter_is_physical_return_and_navigation_keys_are_extended() {
        // Safety: key_event initializes the keyboard member of every INPUT union.
        unsafe {
            let down = key_event(0x0D, false).Anonymous.ki;
            let up = key_event(0x0D, true).Anonymous.ki;
            assert_eq!(down.wScan, 0x1C);
            assert_eq!(down.dwFlags, KEYEVENTF_SCANCODE);
            assert_eq!(up.dwFlags, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);
            assert_eq!(
                key_event(0x25, false).Anonymous.ki.dwFlags,
                KEYEVENTF_EXTENDEDKEY
            );
        }
    }

    #[test]
    fn partial_combination_releases_only_unpaired_injected_keys() {
        let inputs = [
            key_event(0x11, false),
            key_event(0x43, false),
            key_event(0x43, true),
            key_event(0x11, true),
        ];
        let releases = pending_releases(&inputs, 3);
        assert_eq!(releases.len(), 1);
        // Safety: pending_releases constructs the initialized keyboard member.
        let key = unsafe { releases[0].Anonymous.ki };
        assert_eq!(key.wVk, VIRTUAL_KEY(0x11));
        assert!(key.dwFlags.contains(KEYEVENTF_KEYUP));
        assert!(pending_releases(&inputs, 0).is_empty());
        assert!(pending_releases(&inputs, 4).is_empty());
    }
}
