//! The first argument of a worker invocation and the tracing it asks for.
use super::super::WorkerTracing;
use ryo_benchmark_contracts::domain::requests::constants::{DIAGNOSTIC_RYO, RUN_RYO};

/// The first argument of a worker invocation.
#[derive(Clone, Copy, Debug)]
pub struct WorkerCommand<'a>(pub &'a str);

impl WorkerCommand<'_> {
    /// The tracing the command asks for: `None` for a command that is not a worker protocol.
    ///
    /// # Examples
    /// ```
    /// use ryo_benchmark::domain::command_line::types::{worker_command::value::WorkerCommand, WorkerTracing};
    /// assert_eq!(WorkerCommand("run-ryo").tracing(), Some(WorkerTracing::Disabled));
    /// ```
    #[must_use]
    pub fn tracing(self) -> Option<WorkerTracing> {
        match self.0 {
            RUN_RYO => Some(WorkerTracing::Disabled),
            DIAGNOSTIC_RYO => Some(WorkerTracing::JsonLines),
            _ => None,
        }
    }
}
