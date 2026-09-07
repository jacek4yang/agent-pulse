//! Built-in action-sequence presets (§21). Presets are pure data — the core
//! executor knows nothing about them, and nothing here is Codex-specific.

use serde::{Deserialize, Serialize};

use super::action::Action;
use super::key::Key;

/// Identifier for a built-in preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// Focus → "continue" → Enter
    Continue,
    /// Focus → "continue" → Enter → wait → Enter (recommended default)
    ContinueConfirm,
    /// Focus → Enter → wait → "continue" → Enter
    ConfirmContinue,
    /// Focus → Enter → wait → Enter
    DoubleEnter,
    /// Focus → Enter
    EmptySubmit,
}

impl Preset {
    pub const ALL: [Preset; 5] = [
        Preset::Continue,
        Preset::ContinueConfirm,
        Preset::ConfirmContinue,
        Preset::DoubleEnter,
        Preset::EmptySubmit,
    ];

    /// Human-readable name for the UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Preset::Continue => "Continue",
            Preset::ContinueConfirm => "Continue + Confirm",
            Preset::ConfirmContinue => "Confirm + Continue",
            Preset::DoubleEnter => "Double Enter",
            Preset::EmptySubmit => "Empty Submit",
        }
    }

    /// The default preset for quick creation.
    pub fn default_preset() -> Preset {
        Preset::ContinueConfirm
    }

    /// Build the ordered action list for this preset.
    /// `confirm_delay_ms` is the wait between the two key presses.
    pub fn actions(&self, confirm_delay_ms: u64) -> Vec<Action> {
        let enter = || Action::PressKey {
            key: Key::Enter,
            count: 1,
            interval_ms: 0,
        };
        match self {
            Preset::Continue => vec![
                Action::FocusTarget,
                Action::TypeText {
                    text: "continue".into(),
                },
                enter(),
            ],
            Preset::ContinueConfirm => vec![
                Action::FocusTarget,
                Action::TypeText {
                    text: "continue".into(),
                },
                enter(),
                Action::Delay {
                    milliseconds: confirm_delay_ms,
                },
                enter(),
            ],
            Preset::ConfirmContinue => vec![
                Action::FocusTarget,
                enter(),
                Action::Delay { milliseconds: 500 },
                Action::TypeText {
                    text: "continue".into(),
                },
                enter(),
            ],
            Preset::DoubleEnter => vec![
                Action::FocusTarget,
                enter(),
                Action::Delay { milliseconds: 500 },
                enter(),
            ],
            Preset::EmptySubmit => vec![Action::FocusTarget, enter()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continue_confirm_matches_spec_example() {
        let actions = Preset::ContinueConfirm.actions(1000);
        assert_eq!(
            actions,
            vec![
                Action::FocusTarget,
                Action::TypeText {
                    text: "continue".into()
                },
                Action::PressKey {
                    key: Key::Enter,
                    count: 1,
                    interval_ms: 0
                },
                Action::Delay { milliseconds: 1000 },
                Action::PressKey {
                    key: Key::Enter,
                    count: 1,
                    interval_ms: 0
                },
            ]
        );
    }

    #[test]
    fn all_presets_start_with_focus() {
        for p in Preset::ALL {
            assert_eq!(p.actions(500)[0], Action::FocusTarget);
        }
    }
}
