//! Builds the matching adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `globset` matching adapter.
#[must_use]
pub(crate) fn globset() -> impl crate::outbound::pattern_matching::interface::PatternMatchingPort {
    super::adapters::globset::adapter::PatternMatchingGlobsetAdapter
}
