//! What the `job_scheduling` port asks of the outside: running a number of jobs at once on the workers of the pool the caller runs on. A job
//! owns what it writes and shares nothing with another but what it borrows to read.

use super::types::Job;

/// What every `job_scheduling` adapter implements.
pub(crate) trait JobSchedulingPort: Send + Sync {
    /// Runs each job once, and returns when every job is done. The jobs run on the workers of the pool the caller runs on; the adapter
    /// starts none of its own.
    fn run(&self, jobs: Vec<Job<'_>>);
}
