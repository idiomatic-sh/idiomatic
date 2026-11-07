//! The scheduler port, implemented with rayon: the pool the edge built, and every worker drains one atomic queue
//! of job numbers, so a worker that finishes early takes the next job and none waits on another.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber, WorkerCount};

/// The pool of threads the adapter runs jobs on, which the adapter owns or shares with the edge that handed it over.
struct Pool(Box<dyn AsRef<rayon::ThreadPool> + Send + Sync>);

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The rayon scheduler adapter: the pool of threads it runs jobs on, one worker on each.
#[derive(Debug)]
pub(crate) struct SchedulerRayonAdapter {
    /// The pool it built or was handed, or why the system refused it.
    pool: Result<Pool, SchedulerError>,
}

impl SchedulerRayonAdapter {
    /// An adapter over a pool of its own: `workers` threads.
    pub(crate) fn owning(workers: WorkerCount) -> Self {
        Self {
            pool: built(workers).map(|pool| Pool(Box::new(pool))),
        }
    }

    /// An adapter over a pool the edge already runs, shared with it.
    pub(crate) fn over(pool: impl AsRef<rayon::ThreadPool> + Send + Sync + 'static) -> Self {
        Self {
            pool: Ok(Pool(Box::new(pool))),
        }
    }

    /// An adapter over the pool the system refused to build.
    #[cfg(test)]
    pub(crate) const fn refused(error: SchedulerError) -> Self {
        Self { pool: Err(error) }
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
        let pool = AsRef::<rayon::ThreadPool>::as_ref(&*pool.0);
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
