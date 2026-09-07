//! Window target model (spec §24). Persisted identity is rich — never just an
//! HWND, which can be invalidated or recycled.

use serde::{Deserialize, Serialize};

/// How the persisted title criterion is applied during matching.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleMatchMode {
    /// Title must equal the criterion exactly.
    Exact,
    /// Title must contain the criterion (case-insensitive).
    #[default]
    Contains,
    /// Title must match the criterion as a regular expression.
    Regex,
    /// Any window of the matched process qualifies (title ignored).
    Any,
}

/// Identity of the window a task acts on.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowTarget {
    /// Last known HWND value (opaque handle address). Treated as a cache hint
    /// only; always re-verified before use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_hwnd: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub title_match_mode: TitleMatchMode,
}

/// A candidate window as produced by enumeration / consumed by matching.
/// Pure data — no platform handles — so matching is unit-testable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowCandidate {
    pub hwnd: u64,
    pub title: String,
    pub process_id: u32,
    pub process_name: String,
    pub executable_path: Option<String>,
    pub is_visible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_round_trips() {
        let t = WindowTarget {
            last_hwnd: Some(0x0001_2345),
            process_id: Some(18244),
            executable_path: Some(r"C:\Program Files\WindowsApps\wt.exe".into()),
            process_name: Some("WindowsTerminal.exe".into()),
            title: Some("Codex — rust-reality".into()),
            title_match_mode: TitleMatchMode::Contains,
        };
        let json = serde_json::to_string(&t).expect("test serialize");
        let back: WindowTarget = serde_json::from_str(&json).expect("test deserialize");
        assert_eq!(back, t);
    }

    #[test]
    fn target_defaults_to_contains_matching() {
        let t: WindowTarget = serde_json::from_str("{}").expect("test deserialize");
        assert_eq!(t.title_match_mode, TitleMatchMode::Contains);
    }
}
