//! Pure window-matching logic (spec §24–§25). Deliberately free of Win32 so
//! it is unit-testable with synthetic candidates.

use crate::error::{AppError, AppResult};
use crate::model::{TitleMatchMode, WindowCandidate, WindowTarget};

/// Does one candidate satisfy the process criteria (if any are set)?
fn process_matches(candidate: &WindowCandidate, target: &WindowTarget) -> bool {
    let name_ok = target
        .process_name
        .as_deref()
        .map(|want| candidate.process_name.eq_ignore_ascii_case(want))
        .unwrap_or(true);
    let pid_ok = target
        .process_id
        .map(|want| candidate.process_id == want)
        .unwrap_or(true);
    name_ok && pid_ok
}

/// Does one candidate satisfy the title criteria for the given match mode?
fn title_matches(candidate: &WindowCandidate, target: &WindowTarget) -> AppResult<bool> {
    if target.title_match_mode == TitleMatchMode::Any {
        return Ok(true);
    }
    let Some(want) = target.title.as_deref() else {
        return Ok(true); // no title criterion set
    };
    match target.title_match_mode {
        TitleMatchMode::Exact => Ok(candidate.title.eq_ignore_ascii_case(want)),
        TitleMatchMode::Contains => Ok(candidate
            .title
            .to_lowercase()
            .contains(&want.to_lowercase())),
        TitleMatchMode::Regex => {
            let re =
                regex::Regex::new(want).map_err(|e| AppError::InvalidTitleRegex(e.to_string()))?;
            Ok(re.is_match(&candidate.title))
        }
        TitleMatchMode::Any => Ok(true),
    }
}

/// Resolve exactly one candidate or fail. Safety rule (D5): multiple matches
/// ⇒ `TargetAmbiguous`, never a guess; zero ⇒ `TargetNotFound`.
pub fn resolve_target(
    candidates: &[WindowCandidate],
    target: &WindowTarget,
) -> AppResult<WindowCandidate> {
    let mut matched: Vec<&WindowCandidate> = Vec::new();
    for candidate in candidates {
        if process_matches(candidate, target) && title_matches(candidate, target)? {
            matched.push(candidate);
        }
    }
    match matched.len() {
        0 => Err(AppError::TargetNotFound),
        1 => Ok(matched[0].clone()),
        _ => Err(AppError::TargetAmbiguous),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(hwnd: u64, title: &str, pid: u32, process: &str) -> WindowCandidate {
        WindowCandidate {
            hwnd,
            title: title.into(),
            process_id: pid,
            process_name: process.into(),
            executable_path: None,
            is_visible: true,
        }
    }

    fn target(title: &str, mode: TitleMatchMode, process: Option<&str>) -> WindowTarget {
        WindowTarget {
            process_name: process.map(Into::into),
            title: Some(title.into()),
            title_match_mode: mode,
            ..Default::default()
        }
    }

    #[test]
    fn exact_title_matches_single() {
        let candidates = vec![
            candidate(1, "Settings", 10, "explorer.exe"),
            candidate(2, "Codex — rust-reality", 11, "WindowsTerminal.exe"),
        ];
        let t = target("Codex — rust-reality", TitleMatchMode::Exact, None);
        let hit = resolve_target(&candidates, &t).expect("resolve");
        assert_eq!(hit.hwnd, 2);
    }

    #[test]
    fn contains_title_is_case_insensitive() {
        let candidates = vec![candidate(7, "Windows Terminal — CODEX", 5, "wt.exe")];
        let t = target("codex", TitleMatchMode::Contains, None);
        assert_eq!(resolve_target(&candidates, &t).expect("resolve").hwnd, 7);
    }

    #[test]
    fn regex_title_matches() {
        let candidates = vec![
            candidate(1, "build #1234 — done", 9, "ci.exe"),
            candidate(2, "Unrelated", 8, "other.exe"),
        ];
        let t = target(r"build #\d+ — done", TitleMatchMode::Regex, None);
        assert_eq!(resolve_target(&candidates, &t).expect("resolve").hwnd, 1);
    }

    #[test]
    fn invalid_regex_is_structured_error() {
        let candidates = vec![candidate(1, "x", 1, "a.exe")];
        let t = target("([unclosed", TitleMatchMode::Regex, None);
        assert!(matches!(
            resolve_target(&candidates, &t),
            Err(AppError::InvalidTitleRegex(_))
        ));
    }

    #[test]
    fn process_name_filters_candidates() {
        let candidates = vec![
            candidate(1, "Terminal", 10, "cmd.exe"),
            candidate(2, "Terminal", 11, "WindowsTerminal.exe"),
        ];
        let t = target(
            "Terminal",
            TitleMatchMode::Exact,
            Some("WindowsTerminal.exe"),
        );
        assert_eq!(resolve_target(&candidates, &t).expect("resolve").hwnd, 2);
    }

    #[test]
    fn process_id_filters_candidates() {
        let candidates = vec![
            candidate(1, "Terminal", 10, "WindowsTerminal.exe"),
            candidate(2, "Terminal", 11, "WindowsTerminal.exe"),
        ];
        let mut t = target("Terminal", TitleMatchMode::Exact, None);
        t.process_id = Some(11);
        assert_eq!(resolve_target(&candidates, &t).expect("resolve").hwnd, 2);
    }

    #[test]
    fn zero_matches_is_target_not_found() {
        let candidates = vec![candidate(1, "Other", 1, "a.exe")];
        let t = target("Codex", TitleMatchMode::Contains, None);
        assert!(matches!(
            resolve_target(&candidates, &t),
            Err(AppError::TargetNotFound)
        ));
    }

    #[test]
    fn multiple_matches_is_target_ambiguous() {
        let candidates = vec![
            candidate(1, "Codex — alpha", 10, "wt.exe"),
            candidate(2, "Codex — beta", 11, "wt.exe"),
        ];
        let t = target("Codex", TitleMatchMode::Contains, None);
        match resolve_target(&candidates, &t) {
            Err(AppError::TargetAmbiguous) => {} // required behavior
            other => panic!("expected ambiguity, got {other:?}"),
        }
    }

    #[test]
    fn any_mode_ignores_title() {
        let candidates = vec![candidate(3, "anything at all", 10, "wt.exe")];
        let t = target("Codex", TitleMatchMode::Any, Some("wt.exe"));
        assert_eq!(resolve_target(&candidates, &t).expect("resolve").hwnd, 3);
    }

    #[test]
    fn same_title_and_process_is_ambiguous_even_one_process() {
        let candidates = vec![
            candidate(1, "Codex", 10, "wt.exe"),
            candidate(2, "Codex", 10, "wt.exe"),
        ];
        let t = target("Codex", TitleMatchMode::Exact, Some("wt.exe"));
        assert!(matches!(
            resolve_target(&candidates, &t),
            Err(AppError::TargetAmbiguous)
        ));
    }
}
