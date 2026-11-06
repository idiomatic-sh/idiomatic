//! Builds the rows adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: the buffer the edge made. A
//! factory builds nothing else, chooses nothing, and is called only by an edge.

/// The collect rows adapter: it keeps every row in `into`, for the caller to take after the run.
#[must_use]
pub fn collect(
    into: super::types::Collector,
) -> impl crate::outbound::row_sink::interface::RowSinkPort {
    super::adapters::collect::adapter::RowSinkCollectAdapter::new(into)
}
