//! Keyboard input injection via SendInput (spec §26, §27). Unicode typing
//! uses KEYEVENTF_UNICODE events; the clipboard is never touched (D3).

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY,
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
fn send(inputs: &[INPUT]) -> AppResult<()> {
    // Safety: inputs is a fully initialized slice of INPUT structs.
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(AppError::InputInjectionFailed);
    }
    Ok(())
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
    let flags = if up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    keyboard_input(vk, 0, flags)
}

/// Type arbitrary Unicode text as KEYEVENTF_UNICODE down/up pairs.
/// Characters outside the BMP are sent as surrogate pairs.
pub fn type_unicode(text: &str) -> AppResult<()> {
    let mut inputs = Vec::with_capacity(text.len() * 2);
    for unit in text.encode_utf16() {
        let flags_down = KEYEVENTF_UNICODE;
        inputs.push(keyboard_input(0, unit, flags_down));
        inputs.push(keyboard_input(0, unit, flags_down | KEYEVENTF_KEYUP));
    }
    if inputs.is_empty() {
        return Ok(());
    }
    send(&inputs)
}

/// Press a key `count` times with an interval between presses.
pub fn press_key(key: &Key, count: u32, interval_ms: u64) -> AppResult<()> {
    let Some(vk) = virtual_key(key) else {
        return Err(AppError::InputInjectionFailed);
    };
    for i in 0..count {
        send(&[key_event(vk, false), key_event(vk, true)])?;
        if i + 1 < count && interval_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        }
    }
    Ok(())
}

/// Press a combination: modifiers down first, then non-modifiers, then
/// release everything in reverse order (Ctrl+C releases C before Ctrl).
pub fn press_combination(keys: &[Key]) -> AppResult<()> {
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
    send(&inputs)
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
}
