//! Task model (spec §17): the central serializable entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::action::Action;
use super::schedule::{MisfirePolicy, Schedule};
use super::target::WindowTarget;

/// A scheduled automation: target window + schedule + ordered actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,

    pub target: WindowTarget,
    pub schedule: Schedule,
    pub actions: Vec<Action>,

    pub misfire_policy: MisfirePolicy,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
}

impl ScheduledTask {
    /// Create a new task, validating the schedule and computing `next_run_at`.
    pub fn new(
        name: impl Into<String>,
        target: WindowTarget,
        schedule: Schedule,
        actions: Vec<Action>,
        misfire_policy: MisfirePolicy,
        now: DateTime<Utc>,
    ) -> crate::error::AppResult<Self> {
        schedule.validate()?;
        let next_run_at = Some(schedule.first_run_at(now)?);
        Ok(Self {
            id: Uuid::new_v4(),
            name: name.into(),
            enabled: true,
            target,
            schedule,
            actions,
            misfire_policy,
            created_at: now,
            updated_at: now,
            last_run_at: None,
            next_run_at,
        })
    }

    /// Re-arm the schedule (used after enable/creation/edit), recomputing
    /// `next_run_at` from `now`.
    pub fn rearm(&mut self, now: DateTime<Utc>) -> crate::error::AppResult<()> {
        self.schedule.validate()?;
        self.next_run_at = Some(self.schedule.first_run_at(now)?);
        self.updated_at = now;
        Ok(())
    }

    /// Task summary for dashboards and tray menus.
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::key::Key;

    fn utc(secs: i64) -> DateTime<Utc> {
        chrono::TimeZone::timestamp_opt(&Utc, secs, 0)
            .single()
            .expect("test ts")
    }

    fn continue_task() -> (ScheduledTask, DateTime<Utc>) {
        let now = utc(1_000_000);
        let task = ScheduledTask::new(
            "Codex 5-hour Continue",
            WindowTarget::default(),
            Schedule::After {
                hours: 5,
                minutes: 5,
                seconds: 0,
            },
            vec![
                Action::FocusTarget,
                Action::TypeText {
                    text: "continue".into(),
                },
                Action::PressKey {
                    key: Key::Enter,
                    count: 1,
                    interval_ms: 0,
                },
                Action::Delay { milliseconds: 1000 },
                Action::PressKey {
                    key: Key::Enter,
                    count: 1,
                    interval_ms: 0,
                },
            ],
            MisfirePolicy::RunImmediately,
            now,
        )
        .expect("test task");
        (task, now)
    }

    #[test]
    fn new_task_gets_future_next_run() {
        let (task, now) = continue_task();
        let next = task.next_run_at.expect("next_run_at set");
        assert!(next > now);
        assert_eq!(
            next - now,
            chrono::Duration::hours(5) + chrono::Duration::minutes(5)
        );
        assert!(task.enabled);
    }

    #[test]
    fn task_round_trips() {
        let (task, _) = continue_task();
        let json = serde_json::to_string(&task).expect("test serialize");
        let back: ScheduledTask = serde_json::from_str(&json).expect("test deserialize");
        assert_eq!(back, task);
    }

    #[test]
    fn invalid_schedule_rejected_on_new() {
        let result = ScheduledTask::new(
            "bad",
            WindowTarget::default(),
            Schedule::Every {
                interval_seconds: 0,
            },
            vec![],
            MisfirePolicy::default(),
            utc(0),
        );
        assert!(matches!(
            result,
            Err(crate::error::AppError::InvalidSchedule(_))
        ));
    }
}
