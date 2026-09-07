//! Execution history model (spec §40). Records are bounded by the store.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Outcome of one task execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ExecutionOutcome {
    Success,
    Failure {
        /// Stable error code (see `AppError::code`).
        error_code: String,
        /// Human-readable explanation.
        error_message: String,
    },
}

/// One historical execution of a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub id: Uuid,
    pub task_id: Uuid,
    pub task_name: String,
    /// When the task was scheduled to fire.
    pub scheduled_at: DateTime<Utc>,
    /// When execution actually started.
    pub started_at: DateTime<Utc>,
    /// When execution finished.
    pub finished_at: DateTime<Utc>,
    /// Human-readable description of the resolved target.
    pub target_description: String,
    pub outcome: ExecutionOutcome,
}

impl HistoryRecord {
    pub fn duration_ms(&self) -> i64 {
        (self.finished_at - self.started_at).num_milliseconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_round_trips() {
        let rec = HistoryRecord {
            id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            task_name: "Codex Continue".into(),
            scheduled_at: chrono::TimeZone::timestamp_opt(&Utc, 100, 0)
                .single()
                .unwrap(),
            started_at: chrono::TimeZone::timestamp_opt(&Utc, 105, 0)
                .single()
                .unwrap(),
            finished_at: chrono::TimeZone::timestamp_opt(&Utc, 107, 0)
                .single()
                .unwrap(),
            target_description: "Windows Terminal".into(),
            outcome: ExecutionOutcome::Failure {
                error_code: "TargetAmbiguous".into(),
                error_message: "multiple windows matched".into(),
            },
        };
        let json = serde_json::to_string(&rec).expect("test serialize");
        let back: HistoryRecord = serde_json::from_str(&json).expect("test deserialize");
        assert_eq!(back, rec);
        assert_eq!(back.duration_ms(), 2000);
    }
}
