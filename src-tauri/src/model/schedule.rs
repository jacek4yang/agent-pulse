//! Schedule model (spec §18). Schedules are declarative; the scheduler
//! computes and persists absolute `next_run_at` timestamps.

use std::time::Duration;

use chrono::{
    DateTime, Datelike, Duration as ChronoDuration, Local, LocalResult, TimeZone, Timelike, Utc,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// How a task is scheduled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Schedule {
    /// Run once, a relative duration from when the task is created/armed.
    After {
        hours: u32,
        minutes: u32,
        seconds: u32,
    },
    /// Run once, at a specific local wall-clock time.
    At {
        /// Local wall-clock components: (year, month, day, hour, minute, second)
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    },
    /// Run repeatedly at a fixed interval.
    Every {
        /// Interval in seconds (e.g. 5 h = 18000). Must be > 0.
        interval_seconds: u64,
    },
}

impl Schedule {
    /// Validate schedule semantics (e.g. non-zero recurring interval).
    pub fn validate(&self) -> AppResult<()> {
        match self {
            Schedule::After {
                hours,
                minutes,
                seconds,
            } => {
                if *hours == 0 && *minutes == 0 && *seconds == 0 {
                    return Err(AppError::InvalidSchedule(
                        "relative delay must be greater than zero".into(),
                    ));
                }
                Ok(())
            }
            Schedule::At {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } => {
                if !(1..=12).contains(month)
                    || !(1..=31).contains(day)
                    || *hour > 23
                    || *minute > 59
                    || *second > 59
                {
                    return Err(AppError::InvalidSchedule(format!(
                        "invalid date/time components {year}-{month}-{day} {hour}:{minute}:{second}"
                    )));
                }
                let dt = Local.with_ymd_and_hms(*year, *month, *day, *hour, *minute, *second);
                if matches!(dt, LocalResult::None) {
                    return Err(AppError::InvalidSchedule(
                        "date/time does not exist in the local timezone".into(),
                    ));
                }
                Ok(())
            }
            Schedule::Every { interval_seconds } => {
                if *interval_seconds == 0 {
                    return Err(AppError::InvalidSchedule(
                        "recurring interval must be greater than zero".into(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Compute the first `next_run_at` (UTC) for this schedule, starting from `from`.
    pub fn first_run_at(&self, from: DateTime<Utc>) -> AppResult<DateTime<Utc>> {
        self.validate()?;
        match self {
            Schedule::After {
                hours,
                minutes,
                seconds,
            } => {
                let d = ChronoDuration::hours(i64::from(*hours))
                    + ChronoDuration::minutes(i64::from(*minutes))
                    + ChronoDuration::seconds(i64::from(*seconds));
                Ok(from + d)
            }
            Schedule::At {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } => match Local.with_ymd_and_hms(*year, *month, *day, *hour, *minute, *second) {
                LocalResult::Single(local) => Ok(local.with_timezone(&Utc)),
                _ => Err(AppError::InvalidSchedule(
                    "date/time does not exist in the local timezone".into(),
                )),
            },
            Schedule::Every { interval_seconds } => Ok(from
                + ChronoDuration::seconds(i64::try_from(*interval_seconds).unwrap_or(i64::MAX))),
        }
    }

    /// Compute the next occurrence after `fired_at` for recurring schedules.
    /// Computed from schedule semantics relative to the *scheduled* time, not
    /// the actual fire time, so drift never accumulates (spec §19).
    pub fn next_after(
        &self,
        scheduled: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> AppResult<DateTime<Utc>> {
        self.validate()?;
        match self {
            Schedule::Every { interval_seconds } => {
                let step = i64::try_from(*interval_seconds).unwrap_or(i64::MAX);
                // Advance in whole steps from the scheduled time until we are
                // strictly in the future relative to `now`. This handles the
                // sleep-like large time jump without drift accumulation.
                let mut next = scheduled + ChronoDuration::seconds(step);
                while next <= now {
                    next += ChronoDuration::seconds(step);
                }
                Ok(next)
            }
            // One-shot schedules have no next occurrence.
            Schedule::After { .. } | Schedule::At { .. } => Err(AppError::InvalidSchedule(
                "one-shot schedules have no next occurrence".into(),
            )),
        }
    }

    /// Convenience constructor for tests/UX: recurring with a `Duration`.
    pub fn every(duration: Duration) -> Self {
        Schedule::Every {
            interval_seconds: duration.as_secs(),
        }
    }
}

/// What to do when a task's scheduled time has already passed at
/// reconciliation (startup, wake-from-sleep, scheduler wake).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MisfirePolicy {
    /// Run as soon as possible after the scheduled time was missed.
    #[default]
    RunImmediately,
    /// Skip the missed occurrence (recurring: jump to the next future slot).
    Skip,
}

/// Extract (year, month, day, hour, minute, second) from a local datetime.
/// Utility for building `Schedule::At` from UI-picked datetimes.
pub fn local_components(dt: DateTime<Local>) -> (i32, u32, u32, u32, u32, u32) {
    (
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).single().expect("test timestamp")
    }

    #[test]
    fn relative_schedule_computes_future_time() {
        let now = utc(1_000_000);
        let s = Schedule::After {
            hours: 5,
            minutes: 5,
            seconds: 0,
        };
        let next = s.first_run_at(now).expect("test schedule");
        assert_eq!(
            next - now,
            ChronoDuration::hours(5) + ChronoDuration::minutes(5)
        );
    }

    #[test]
    fn zero_relative_delay_is_invalid() {
        let s = Schedule::After {
            hours: 0,
            minutes: 0,
            seconds: 0,
        };
        assert!(matches!(
            s.first_run_at(utc(0)),
            Err(AppError::InvalidSchedule(_))
        ));
    }

    #[test]
    fn recurring_zero_interval_is_invalid() {
        let s = Schedule::Every {
            interval_seconds: 0,
        };
        assert!(matches!(s.validate(), Err(AppError::InvalidSchedule(_))));
    }

    #[test]
    fn recurring_next_after_is_drift_free() {
        let start = utc(0);
        let s = Schedule::every(Duration::from_secs(60));
        // Simulate a sleep-like jump: 10 intervals pass before reconciliation.
        let now = utc(601);
        let next = s.next_after(start, now).expect("test schedule");
        // Must land on the next multiple-of-60 boundary past now (660), not
        // start + 60 (which would refire immediately and accumulate drift).
        assert_eq!(next, utc(660));
    }

    #[test]
    fn recurring_next_after_small_lateness() {
        let start = utc(0);
        let s = Schedule::every(Duration::from_secs(60));
        let now = utc(61); // fired 1s late
        let next = s.next_after(start, now).expect("test schedule");
        assert_eq!(next, utc(120));
    }

    #[test]
    fn one_shot_has_no_next() {
        let s = Schedule::After {
            hours: 1,
            minutes: 0,
            seconds: 0,
        };
        assert!(matches!(
            s.next_after(utc(0), utc(1)),
            Err(AppError::InvalidSchedule(_))
        ));
    }

    #[test]
    fn schedule_round_trips() {
        for s in [
            Schedule::After {
                hours: 5,
                minutes: 5,
                seconds: 0,
            },
            Schedule::Every {
                interval_seconds: 1800,
            },
            Schedule::At {
                year: 2026,
                month: 9,
                day: 7,
                hour: 9,
                minute: 30,
                second: 0,
            },
        ] {
            let json = serde_json::to_string(&s).expect("test serialize");
            let back: Schedule = serde_json::from_str(&json).expect("test deserialize");
            assert_eq!(back, s);
        }
    }

    #[test]
    fn misfire_policy_defaults_to_run_immediately() {
        assert_eq!(MisfirePolicy::default(), MisfirePolicy::RunImmediately);
    }
}
