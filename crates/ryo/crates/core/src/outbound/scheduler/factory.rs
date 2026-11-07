//! Builds the scheduler adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use crate::outbound::scheduler::types::WorkerCount;

/// The rayon scheduler adapter over a pool of threads of its own: `workers` of them. A pool the system refuses is answered by the first
/// run over it.
#[must_use]
pub fn rayon(
    workers: WorkerCount,
) -> impl crate::outbound::scheduler::interface::SchedulerPort {
    super::adapters::rayon::adapter::SchedulerRayonAdapter::owning(workers)
}

/// The rayon scheduler adapter over a pool the edge already runs and shares with it, which the scheduler uses and never copies.
#[must_use]
pub fn rayon_over<P: AsRef<rayon::ThreadPool> + Send + Sync + 'static>(
    pool: P,
) -> impl crate::outbound::scheduler::interface::SchedulerPort {
    super::adapters::rayon::adapter::SchedulerRayonAdapter::over(pool)
}

/// The sequential scheduler adapter, from what the edge passes.
#[must_use]
pub fn sequential() -> impl crate::outbound::scheduler::interface::SchedulerPort {
    super::adapters::sequential::adapter::SchedulerSequentialAdapter
}
