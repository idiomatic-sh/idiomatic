//! Builds the matching adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The globset matching adapter, from what the edge passes.
#[must_use]
pub fn globset() -> impl crate::outbound::matching::interface::MatchingPort {
    super::adapters::globset::adapter::MatchingGlobsetAdapter
}
