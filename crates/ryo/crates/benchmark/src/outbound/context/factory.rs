//! Builds the context adapters, one function each, by constructor injection: a factory takes
//! the clients and settings the edge opened and returns the adapter built with them, so no
//! port method takes configuration. A factory builds nothing else, chooses nothing, and is
//! called only by an edge.

/// The local filesystem context adapter.
#[must_use]
pub fn local() -> impl crate::outbound::context::interface::ContextPort {
    super::adapters::local::adapter::ContextLocalAdapter
}
