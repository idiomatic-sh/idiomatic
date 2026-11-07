//! Builds the profile adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The profile adapter that keeps one profile for each thread.
#[must_use]
pub fn thread<State: Send + std::fmt::Debug + 'static>() -> impl crate::outbound::profile::interface::ProfilePort<State> {
    super::adapters::thread::adapter::ProfileThreadAdapter::new()
}
