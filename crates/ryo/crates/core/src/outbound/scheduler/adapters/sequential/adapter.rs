//! The scheduler port, implemented in order on the calling thread, on one worker.

use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber};

/// The sequential scheduler adapter.
#[derive(Debug)]
pub(crate) struct SchedulerSequentialAdapter;

impl SchedulerPort for SchedulerSequentialAdapter {
    fn run(
        &self,
        workers: &mut [Box<dyn FnMut(JobNumber) + Send + '_>],
        count: JobCount,
    ) -> Result<(), SchedulerError> {
        let Some(worker) = workers.first_mut() else {
            return Err(SchedulerError::NoWorkers);
        };
        for index in 0..count.0 {
            worker(JobNumber(index));
        }
        Ok(())
    }
}
