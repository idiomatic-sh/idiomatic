//! Builds the scheduler adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use crate::outbound::scheduler::types::WorkerCount;

/// How many workers the pool `pool(asked)` builds runs jobs on, which the use case that calls it makes one worker state for each.
// ignore: idiomatic.arch/port/factory-name - the count is no adapter but the size of the pool the edge builds once with `pool`: it is a fact the use case that calls it reads, which the rayon adapter alone can answer
#[must_use]
pub fn workers(asked: ryo_values::domain::runs::types::WorkersAsked) -> WorkerCount {
    WorkerCount(
        ryo_values::domain::runs::service::workers_to_start(
            asked,
            super::adapters::rayon::adapter::machine(),
        )
        .0,
    )
}

/// The invocation's one pool of threads: `asked` of them, at most as many as the machine has, and as many as it has when none is
/// asked; which the edge builds once and hands to `rayon`. A pool the system refuses is answered by the first run over it.
///
/// # Errors
///
/// `Unavailable` when the system refuses the threads, `NoWorkers` when none can run.
// ignore: idiomatic.arch/idiomatic-rust/no-sync-in-public-api - the pool is shared by the edge, which builds it once and injects it into the scheduler and into the engine: an `Arc` is what shares it
// ignore: idiomatic.arch/port/factory-name - the pool is no adapter but the one resource the edge builds once and injects into `rayon` and into the engine; its threads are the rayon adapter's to size
pub fn pool(
    asked: ryo_values::domain::runs::types::WorkersAsked,
) -> Result<std::sync::Arc<rayon::ThreadPool>, crate::outbound::scheduler::errors::SchedulerError> {
    super::adapters::rayon::adapter::built(WorkerCount(
        ryo_values::domain::runs::service::workers_to_start(
            asked,
            super::adapters::rayon::adapter::machine(),
        )
        .0,
    ))
}

/// The rayon scheduler adapter over the pool the edge built and runs, shared with it.
#[must_use]
pub fn rayon(
    // ignore: idiomatic.arch/idiomatic-rust/no-sync-in-public-api - the pool is the caller's, which it runs its own work on too: the scheduler holds a share of it, never a copy
    pool: Result<
        std::sync::Arc<rayon::ThreadPool>,
        crate::outbound::scheduler::errors::SchedulerError,
    >,
) -> impl crate::outbound::scheduler::interface::SchedulerPort {
    super::adapters::rayon::adapter::SchedulerRayonAdapter::new(pool)
}

/// The sequential scheduler adapter, from what the edge passes.
#[must_use]
pub fn sequential() -> impl crate::outbound::scheduler::interface::SchedulerPort {
    super::adapters::sequential::adapter::SchedulerSequentialAdapter
}
