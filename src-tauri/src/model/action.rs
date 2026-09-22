//! Action model (spec §20): a generic, ordered sequence of automation steps.
//! The engine is deliberately product-agnostic — nothing here knows about
//! Codex or any specific terminal agent.

use serde::{Deserialize, Serialize};

use super::key::Key;

/// One step in an automation sequence. Actions execute strictly in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Bring the resolved target window to the foreground.
    FocusTarget,
    /// Restore the target window if it is minimized.
    RestoreTarget,
    /// Type arbitrary Unicode text via keyboard events.
    TypeText { text: String },
    /// Press a key one or more times with an interval between presses.
    PressKey {
        key: Key,
        count: u32,
        interval_ms: u64,
    },
    /// Press a key combination (e.g. Ctrl+C) with press-and-hold semantics.
    KeyCombination { keys: Vec<Key> },
    /// Wait for the given duration.
    Delay { milliseconds: u64 },
    /// Show a desktop notification.
    Notify { message: String },
}

impl Action {
    pub fn validate(&self) -> crate::error::AppResult<()> {
        use crate::error::AppError;
        let valid_key = |key: &Key| match key {
            Key::Letter(c) => c.is_ascii_alphabetic(),
            Key::Digit(d) => *d <= 9,
            Key::Function(n) => (1..=12).contains(n),
            _ => true,
        };
        let valid = match self {
            // Control characters could submit a command during a text step.
            // Require an explicit PressKey action for every Enter/Tab instead.
            Self::TypeText { text } => text.len() <= 16384 && !text.chars().any(char::is_control),
            Self::PressKey {
                key,
                count,
                interval_ms,
            } => {
                valid_key(key)
                    && key.as_modifier().is_none()
                    && (1..=1000).contains(count)
                    && *interval_ms <= 60000
            }
            Self::KeyCombination { keys } => {
                !keys.is_empty()
                    && keys.len() <= 8
                    && keys.iter().all(valid_key)
                    && keys.iter().any(|key| key.as_modifier().is_none())
                    && keys
                        .iter()
                        .enumerate()
                        .all(|(i, key)| !keys[..i].contains(key))
            }
            Self::Delay { milliseconds } => *milliseconds <= 3600000,
            Self::Notify { message } => message.len() <= 16384,
            Self::FocusTarget | Self::RestoreTarget => true,
        };
        if valid {
            Ok(())
        } else {
            Err(AppError::InvalidAction("invalid key/count/delay, oversized text, or control character in text; use an explicit PressKey for Enter".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_round_trips() {
        for action in [
            Action::FocusTarget,
            Action::RestoreTarget,
            Action::TypeText {
                text: "continue ✓".into(),
            },
            Action::PressKey {
                key: Key::Enter,
                count: 1,
                interval_ms: 0,
            },
            Action::KeyCombination {
                keys: vec![Key::Ctrl, Key::Letter('c')],
            },
            Action::Delay { milliseconds: 1000 },
            Action::Notify {
                message: "done".into(),
            },
        ] {
            let json = serde_json::to_string(&action).expect("test serialize");
            let back: Action = serde_json::from_str(&json).expect("test deserialize");
            assert_eq!(back, action);
        }
    }

    #[test]
    fn action_tag_is_stable_for_frontend() {
        let json = serde_json::to_value(Action::Delay { milliseconds: 5 }).expect("test serialize");
        assert_eq!(json["type"], "delay");
        assert_eq!(json["milliseconds"], 5);
    }
}
