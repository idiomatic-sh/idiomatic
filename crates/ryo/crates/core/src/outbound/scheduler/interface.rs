//! What the `scheduler` port asks of the outside: running a number of jobs on workers, each of
//! which owns what it works on, so no job shares anything with another.

use super::errors::SchedulerError;
use super::types::{JobCount, JobNumber};

/// What every `scheduler` adapter implements.
pub trait SchedulerPort: Send + Sync + std::fmt::Debug {
    /// Runs one job for each index below `count`, each on one of `workers`, and returns when every
    /// job is done. A worker is called by one thread at a time and keeps what it owns between jobs.
    ///
    /// # Errors
    ///
    /// `Unavailable` when the workers cannot be started, `NoWorkers` when none was given.
    fn run(
        &self,
        workers: &mut [Box<dyn FnMut(JobNumber) + Send + '_>],
        count: JobCount,
    ) -> Result<(), SchedulerError>;
}
