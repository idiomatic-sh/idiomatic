//! Builds the scheduler adapters, one function each. A factory builds nothing else, chooses nothing, and is called only
//! by an edge.

/// The rayon scheduler over `pool`, the one pool the edge built for the invocation.
#[must_use]
pub fn rayon(pool: super::types::WorkerPool) -> impl super::interface::SchedulerPort {
    super::adapters::rayon::adapter::SchedulerRayonAdapter { pool }
}

/// The invocation's one pool of `workers` workers, which the edge builds once and injects into every consumer.
// ignore: idiomatic.arch/port/factory-name - the pool is no adapter but the one resource the edge builds once and injects into `rayon` and into ryo's engine; its workers' stack is the rayon adapter's to size
#[must_use]
pub fn pool(workers: super::types::WorkerCount) -> super::types::WorkerPool {
    super::adapters::rayon::adapter::built(workers)
}
