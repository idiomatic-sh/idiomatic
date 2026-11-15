//! Builds the scheduler adapter by constructor injection; called only by an edge.

/// The adapter that runs each task on a thread of its own.
#[must_use]
pub fn threads() -> impl super::interface::SchedulerPort {
    super::adapters::threads::adapter::SchedulerThreadsAdapter
}
