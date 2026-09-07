//! Scheduler: owns firing decisions; the frontend never schedules.

pub mod clock;
pub mod engine;

pub use clock::{Clock, SystemClock};
pub use engine::{Decision, ReconcileReport, Scheduler, compute_next_run, decide, reconcile};
