//! What the `scheduler` port asks of the outside: workers for a set of tasks; it never takes configuration.

/// What every `scheduler` adapter implements.
pub trait SchedulerPort: Send + Sync {
    /// Run every task, each to completion exactly once, and return when all are done; the tasks may run at once, so one
    /// that waits for another's result must be given its own worker. A task owns or borrows what it works on and reports
    /// through what it captured.
    fn run_all(&self, tasks: Vec<Box<dyn FnOnce() + Send + '_>>);
}
