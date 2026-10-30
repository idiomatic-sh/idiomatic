//! Builds the bmf adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

/// The JSON adapter: Bencher Metric Format read as JSON.
#[must_use]
pub fn json() -> impl crate::outbound::bmf::interface::BmfPort {
    super::adapters::json::adapter::BmfJsonAdapter
}
