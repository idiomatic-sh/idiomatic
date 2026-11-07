//! The scheduler port, implemented over the rayon pool the edge built once and injected.
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{WorkerCount, WorkerPool};

/// The rayon scheduler adapter: the pool the edge built, which every run installs its job in.
#[derive(Debug)]
pub(crate) struct SchedulerRayonAdapter {
    /// The pool every run installs its job in, or why it could not be built.
    pub pool: WorkerPool,
}

impl SchedulerPort for SchedulerRayonAdapter {
    fn run(&self, job: &mut (dyn FnMut() + Send)) -> Result<(), SchedulerError> {
        self.pool.0.as_ref().map_err(Clone::clone)?.install(job);
        Ok(())
    }
}

/// The pool of `workers` workers, at least one, each on the stack the work needs; or why the system refused it.
#[must_use]
pub(crate) fn built(workers: WorkerCount) -> WorkerPool {
    WorkerPool(
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers.0.max(1))
            .stack_size(super::constants::STACK_BYTES)
            .build()
            .map(std::sync::Arc::new)
            .map_err(|error| SchedulerError::Unavailable {
                reason: error.to_string(),
            }),
    )
}
