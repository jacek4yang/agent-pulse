//! Executor: ordered, safe execution of action sequences.

pub mod action_executor;
pub mod automation;

pub use action_executor::{
    ExecutionGuard, execute_task, history_record, run_task_once, run_task_once_cancellable,
};
pub use automation::PlatformAutomation;
