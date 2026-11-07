//! Builds the matching adapters, one function each. A factory builds nothing else, chooses nothing, and is called only
//! by an edge.

/// The globset adapter.
#[must_use]
pub fn globset() -> impl crate::outbound::matching::interface::MatchingPort {
    super::adapters::globset::adapter::MatchingGlobsetAdapter
}
