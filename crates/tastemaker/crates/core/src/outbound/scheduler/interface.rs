//! Running the invocation's parallel work in its own bounded pool.
use super::errors::SchedulerError;

/// Runs work inside the invocation's pool.
pub trait SchedulerPort: Send + Sync {
    /// Runs `job` once, inside the invocation's pool: the parallel work it starts runs on the pool's workers and no
    /// other's.
    /// # Errors
    /// Returns a failure when the pool cannot be started.
    fn run(&self, job: &mut (dyn FnMut() + Send)) -> Result<(), SchedulerError>;
}
