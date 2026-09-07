//! Domain model: tasks, schedules, actions, targets, history, settings.

pub mod action;
pub mod history;
pub mod key;
pub mod presets;
pub mod schedule;
pub mod settings;
pub mod target;
pub mod task;

pub use action::Action;
pub use history::{ExecutionOutcome, HistoryRecord};
pub use key::Key;
pub use schedule::{MisfirePolicy, Schedule};
pub use settings::{Settings, Theme};
pub use target::{TitleMatchMode, WindowCandidate, WindowTarget};
pub use task::ScheduledTask;
