//! Strongly-typed keyboard keys (spec §28). No raw virtual-key constants in
//! business logic; the Win32 backend maps `Key` → VIRTUAL_KEY.

use serde::{Deserialize, Serialize};

/// Modifier keys usable in combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
}

/// A physical key or modifier usable in actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Ctrl,
    Shift,
    Alt,
    Letter(char),
    Digit(u8),
    Function(u8),
}

impl Key {
    /// All keys representable as modifiers for combination handling.
    pub fn as_modifier(&self) -> Option<Modifier> {
        match self {
            Key::Ctrl => Some(Modifier::Ctrl),
            Key::Shift => Some(Modifier::Shift),
            Key::Alt => Some(Modifier::Alt),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_round_trips() {
        for key in [
            Key::Enter,
            Key::Space,
            Key::Up,
            Key::Ctrl,
            Key::Letter('a'),
            Key::Letter('Z'),
            Key::Digit(0),
            Key::Digit(9),
            Key::Function(12),
        ] {
            let json = serde_json::to_string(&key).expect("test serialize");
            let back: Key = serde_json::from_str(&json).expect("test deserialize");
            assert_eq!(back, key);
        }
    }
}
