//! Schedule model (spec §18). Schedules are declarative; the scheduler
//! computes and persists absolute `next_run_at` timestamps.

use std::str::FromStr;
use std::time::Duration;

use chrono::{
    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, Local, LocalResult, TimeZone,
    Timelike, Utc,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Default timezone for `Schedule::At` (system local zone).
fn default_timezone() -> String {
    "local".to_string()
}

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
    /// Run once, at a specific wall-clock time in a named timezone.
    At {
        /// Wall-clock components in `timezone`: (year, month, day, hour, minute, second)
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        /// IANA timezone name ("Asia/Shanghai"), "utc", or "local".
        /// Defaults to "local" so schedules persisted before this field
        /// existed keep their meaning (no migration needed).
        #[serde(default = "default_timezone")]
        timezone: String,
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
                timezone,
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
                let tz = resolve_timezone(timezone)?;
                let dt = tz.with_ymd_and_hms(*year, *month, *day, *hour, *minute, *second);
                if matches!(dt, LocalResult::None) {
                    return Err(AppError::InvalidSchedule(format!(
                        "date/time does not exist in timezone '{timezone}' (DST gap?)"
                    )));
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
                timezone,
            } => {
                let tz = resolve_timezone(timezone)?;
                match tz.with_ymd_and_hms(*year, *month, *day, *hour, *minute, *second) {
                    LocalResult::Single(local) => Ok(local.with_timezone(&Utc)),
                    _ => Err(AppError::InvalidSchedule(format!(
                        "date/time does not exist in timezone '{timezone}' (DST gap?)"
                    ))),
                }
            }
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

/// A resolved timezone reference for `Schedule::At`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TzKind {
    /// The machine's system timezone.
    Local,
    /// A fixed UTC offset (e.g. UTC+08:00).
    Fixed(FixedOffset),
    /// A named IANA zone (handles DST correctly).
    Named(chrono_tz::Tz),
}

impl TzKind {
    /// Resolve wall-clock components in this zone to a UTC instant.
    pub fn with_ymd_and_hms(
        &self,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> LocalResult<DateTime<Utc>> {
        match self {
            TzKind::Local => Local
                .with_ymd_and_hms(year, month, day, hour, minute, second)
                .map(|dt| dt.with_timezone(&Utc)),
            TzKind::Fixed(offset) => offset
                .with_ymd_and_hms(year, month, day, hour, minute, second)
                .map(|dt| dt.with_timezone(&Utc)),
            TzKind::Named(tz) => tz
                .with_ymd_and_hms(year, month, day, hour, minute, second)
                .map(|dt| dt.with_timezone(&Utc)),
        }
    }
}

/// Resolve a timezone string: "local"/"" (system), "utc", fixed offsets
/// like "UTC+8" / "GMT-05:30", or any IANA name ("Asia/Shanghai").
pub fn resolve_timezone(name: &str) -> AppResult<TzKind> {
    let n = name.trim();
    if n.is_empty() || n.eq_ignore_ascii_case("local") || n.eq_ignore_ascii_case("system") {
        return Ok(TzKind::Local);
    }
    if n.eq_ignore_ascii_case("utc") || n.eq_ignore_ascii_case("z") || n.eq_ignore_ascii_case("gmt")
    {
        return Ok(TzKind::Named(chrono_tz::UTC));
    }
    // Fixed offsets: UTC+8, GMT-05:30, +09:00 …
    let offset_re =
        regex::Regex::new(r"^(?:utc|gmt)?([+-])(\d{1,2})(?::?(\d{2}))?$").expect("static regex");
    if let Some(caps) = offset_re.captures(&n.to_lowercase()) {
        let sign: i32 = if &caps[1] == "-" { -1 } else { 1 };
        let hours: i32 = caps[2].parse().unwrap_or(0);
        let minutes: i32 = caps
            .get(3)
            .map(|m| m.as_str().parse().unwrap_or(0))
            .unwrap_or(0);
        if hours > 23 || minutes > 59 {
            return Err(AppError::InvalidSchedule(format!(
                "invalid UTC offset '{name}'"
            )));
        }
        return FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
            .map(TzKind::Fixed)
            .ok_or_else(|| AppError::InvalidSchedule(format!("invalid UTC offset '{name}'")));
    }
    chrono_tz::Tz::from_str(n)
        .map(TzKind::Named)
        .map_err(|_| {
            AppError::InvalidSchedule(format!(
                "unknown timezone '{name}' (use an IANA name like Asia/Shanghai, a UTC offset like UTC+8, or 'local')"
            ))
        })
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
                timezone: "Asia/Shanghai".into(),
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

    // --- timezone resolution ---

    fn at(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        timezone: &str,
    ) -> Schedule {
        Schedule::At {
            year,
            month,
            day,
            hour,
            minute,
            second,
            timezone: timezone.into(),
        }
    }

    #[test]
    fn at_in_named_timezone_converts_to_utc() {
        // 2026-09-08 01:26:05 Asia/Shanghai (UTC+8) == 2026-09-07T17:26:05Z
        let s = at(2026, 9, 8, 1, 26, 5, "Asia/Shanghai");
        let at = s.first_run_at(utc(0)).expect("test schedule");
        assert_eq!(at, utc(1_788_801_965));
    }

    #[test]
    fn at_utc_and_gmt_are_equivalent() {
        let a = at(2026, 9, 8, 12, 0, 0, "utc")
            .first_run_at(utc(0))
            .expect("utc");
        let b = at(2026, 9, 8, 12, 0, 0, "GMT")
            .first_run_at(utc(0))
            .expect("gmt");
        assert_eq!(a, b);
    }

    #[test]
    fn at_fixed_offset_parses() {
        // UTC+8 matches Asia/Shanghai on a winter date (no DST either side).
        let named = at(2026, 1, 15, 8, 0, 0, "Asia/Shanghai")
            .first_run_at(utc(0))
            .expect("named");
        let fixed = at(2026, 1, 15, 8, 0, 0, "UTC+8")
            .first_run_at(utc(0))
            .expect("fixed");
        assert_eq!(named, fixed);
    }

    #[test]
    fn unknown_timezone_is_structured_error() {
        let s = at(2026, 9, 8, 1, 0, 0, "Mars/Olympus");
        assert!(matches!(s.validate(), Err(AppError::InvalidSchedule(_))));
    }

    #[test]
    fn dst_gap_is_invalid_schedule() {
        // 02:30 does not exist on the US spring-forward night (2026-03-08).
        let s = at(2026, 3, 8, 2, 30, 0, "America/New_York");
        assert!(matches!(
            s.first_run_at(utc(0)),
            Err(AppError::InvalidSchedule(_))
        ));
    }

    #[test]
    fn legacy_at_json_without_timezone_defaults_to_local() {
        let json = r#"{"kind":"at","year":2026,"month":9,"day":8,"hour":1,"minute":26,"second":5}"#;
        let s: Schedule = serde_json::from_str(json).expect("test deserialize");
        match s {
            Schedule::At { ref timezone, .. } => assert_eq!(timezone, "local"),
            other => panic!("expected At, got {other:?}"),
        }
        // And it validates via the local zone.
        assert!(s.validate().is_ok());
    }
}
