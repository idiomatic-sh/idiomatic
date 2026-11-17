//! Builds the host adapter by constructor injection; called only by an edge.

/// The adapter that reads this machine's kernel and container markers.
#[must_use]
pub fn local() -> impl super::interface::HostPort {
    super::adapters::local::adapter::HostLocalAdapter
}
