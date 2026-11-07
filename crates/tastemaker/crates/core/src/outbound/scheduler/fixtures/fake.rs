//! The serial scheduler: every job runs in a pool of one worker.
use super::super::{errors::SchedulerError, interface::SchedulerPort};

/// Deterministic one-worker fixture.
#[derive(Clone, Copy, Debug)]
pub struct SchedulerFake;

impl SchedulerPort for SchedulerFake {
    fn run(&self, job: &mut (dyn FnMut() + Send)) -> Result<(), SchedulerError> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .map_err(|error| SchedulerError::Unavailable {
                reason: error.to_string(),
            })?;
        pool.install(job);
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
