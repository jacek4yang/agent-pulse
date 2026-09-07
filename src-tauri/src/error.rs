//! Structured error model shared across the application core.
//!
//! Every error carries a stable machine-readable `code` (consumed by the
//! frontend) plus a human-readable message. No error strings are scattered
//! ad-hoc through the codebase.

use serde::Serialize;

/// Typed application errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// No window matched the target criteria.
    #[error("no window matched the target criteria")]
    TargetNotFound,
    /// Multiple windows matched; refusing to guess.
    #[error("multiple windows matched the target criteria; refine the matching")]
    TargetAmbiguous,
    /// The matched window is not visible.
    #[error("the target window is not visible")]
    TargetNotVisible,
    /// The target could not be brought to the foreground.
    #[error("target window could not be activated; no keyboard input was sent")]
    FailedToActivateTarget,
    /// The target lost focus during a sequence; input aborted.
    #[error("target lost focus during execution; input aborted")]
    TargetLostFocus,
    /// A SendInput (or equivalent) call failed.
    #[error("keyboard input injection failed")]
    InputInjectionFailed,
    /// A schedule description is invalid (e.g. zero interval, past deadline).
    #[error("invalid schedule: {0}")]
    InvalidSchedule(String),
    /// Persistence layer failed (read/write/corruption without fallback).
    #[error("persistence failure: {0}")]
    PersistenceFailure(String),
    /// The task is disabled and cannot run.
    #[error("task is disabled")]
    TaskDisabled,
    /// Another task is already executing a sequence.
    #[error("another task execution is already running")]
    ExecutionAlreadyRunning,
    /// Task with the given id does not exist.
    #[error("task not found: {0}")]
    TaskNotFound(uuid::Uuid),
    /// Window title match mode was given an invalid regex.
    #[error("invalid title regex: {0}")]
    InvalidTitleRegex(String),
}

impl AppError {
    /// Stable machine-readable code for the frontend / history records.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::TargetNotFound => "TargetNotFound",
            AppError::TargetAmbiguous => "TargetAmbiguous",
            AppError::TargetNotVisible => "TargetNotVisible",
            AppError::FailedToActivateTarget => "FailedToActivateTarget",
            AppError::TargetLostFocus => "TargetLostFocus",
            AppError::InputInjectionFailed => "InputInjectionFailed",
            AppError::InvalidSchedule(_) => "InvalidSchedule",
            AppError::PersistenceFailure(_) => "PersistenceFailure",
            AppError::TaskDisabled => "TaskDisabled",
            AppError::ExecutionAlreadyRunning => "ExecutionAlreadyRunning",
            AppError::TaskNotFound(_) => "TaskNotFound",
            AppError::InvalidTitleRegex(_) => "InvalidTitleRegex",
        }
    }
}

/// Wire representation sent to the frontend over Tauri IPC.
#[derive(Debug, Clone, Serialize)]
pub struct SerializedError {
    pub code: String,
    pub message: String,
}

impl From<&AppError> for SerializedError {
    fn from(e: &AppError) -> Self {
        SerializedError {
            code: e.code().to_string(),
            message: e.to_string(),
        }
    }
}

impl From<AppError> for SerializedError {
    fn from(e: AppError) -> Self {
        SerializedError::from(&e)
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedError::from(self).serialize(serializer)
    }
}

/// Convenient alias for fallible application operations.
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(AppError::TargetAmbiguous.code(), "TargetAmbiguous");
        assert_eq!(
            AppError::InvalidSchedule("x".into()).code(),
            "InvalidSchedule"
        );
    }

    #[test]
    fn error_serializes_with_code_and_message() {
        let json = serde_json::to_value(AppError::TargetNotFound).expect("test serialize");
        assert_eq!(json["code"], "TargetNotFound");
        assert!(json["message"].as_str().is_some_and(|m| !m.is_empty()));
    }
}
