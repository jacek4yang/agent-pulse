//! Application settings (§37–§39).

use serde::{Deserialize, Serialize};

use super::schedule::MisfirePolicy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    System,
    Light,
    Dark,
}

/// UI language. `System` follows the OS locale at startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    System,
    En,
    Zh,
}

/// Persisted user settings. Serialized as part of the versioned store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    /// UI language (English or Simplified Chinese).
    pub language: Language,
    /// Notify on successful executions.
    pub notify_on_success: bool,
    /// Notify on failed executions.
    pub notify_on_failure: bool,
    /// Start Agent Pulse when Windows starts.
    pub start_with_windows: bool,
    /// Start with the main window hidden.
    pub start_minimized: bool,
    /// Default misfire policy offered for new tasks.
    pub default_misfire_policy: MisfirePolicy,
    /// Maximum number of history records retained.
    pub history_limit: u32,
    /// Abort a running sequence if the target loses focus (recommended).
    pub abort_on_focus_loss: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            language: Language::System,
            notify_on_success: true,
            notify_on_failure: true,
            start_with_windows: false,
            start_minimized: false,
            default_misfire_policy: MisfirePolicy::RunImmediately,
            history_limit: 1000,
            abort_on_focus_loss: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_round_trips() {
        for l in [Language::System, Language::En, Language::Zh] {
            let json = serde_json::to_string(&l).expect("test serialize");
            let back: Language = serde_json::from_str(&json).expect("test deserialize");
            assert_eq!(back, l);
        }
    }

    #[test]
    fn settings_round_trip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).expect("test serialize");
        let back: Settings = serde_json::from_str(&json).expect("test deserialize");
        assert_eq!(back, s);
    }

    #[test]
    fn partial_settings_json_fills_defaults() {
        let back: Settings =
            serde_json::from_str(r#"{"theme": "dark"}"#).expect("test deserialize");
        assert_eq!(back.theme, Theme::Dark);
        assert_eq!(back.history_limit, 1000);
        assert!(back.abort_on_focus_loss);
    }
}
