//! The scheduler port, implemented with rayon: the pool the edge built, and every worker drains one atomic queue
//! of job numbers, so a worker that finishes early takes the next job and none waits on another.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber, WorkerCount};

/// The rayon scheduler adapter: the pool of threads it runs jobs on, one worker on each.
#[derive(Debug)]
pub(crate) struct SchedulerRayonAdapter {
    /// The pool its factory built or was handed, or why the system refused it.
    pool: Result<std::sync::Arc<rayon::ThreadPool>, SchedulerError>,
}

impl SchedulerRayonAdapter {
    /// An adapter over `pool`.
    pub(crate) const fn new(
        pool: Result<std::sync::Arc<rayon::ThreadPool>, SchedulerError>,
    ) -> Self {
        Self { pool }
    }
}

impl SchedulerPort for SchedulerRayonAdapter {
    fn run(
        &self,
        workers: &mut [Box<dyn FnMut(JobNumber) + Send + '_>],
        count: JobCount,
    ) -> Result<(), SchedulerError> {
        if workers.is_empty() {
            return Err(SchedulerError::NoWorkers);
        }
        let pool = self.pool.as_ref().map_err(Clone::clone)?;
        let next = AtomicU32::new(0);
        pool.scope(|scope| {
            for worker in workers.iter_mut() {
                let next = &next;
                scope.spawn(move |_| {
                    loop {
                        let job = next.fetch_add(1, Ordering::Relaxed);
                        if job >= count.0 {
                            break;
                        }
                        worker(JobNumber(job));
                    }
                });
            }
        });
        Ok(())
    }
}

/// How many threads the machine can run at once.
pub(crate) fn machine() -> ryo_values::domain::runs::types::MachineWorkers {
    ryo_values::domain::runs::types::MachineWorkers(
        u32::try_from(std::thread::available_parallelism().map_or(1, std::num::NonZero::get))
            .unwrap_or(1),
    )
}

/// The pool of `workers` threads; or why the system refused it.
pub(crate) fn built(
    workers: WorkerCount,
) -> Result<std::sync::Arc<rayon::ThreadPool>, SchedulerError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(usize::try_from(workers.0).unwrap_or(1))
        .build()
        .map(std::sync::Arc::new)
        .map_err(|error| SchedulerError::Unavailable {
            reason: error.to_string(),
        })
}
