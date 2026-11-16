//! Builds the sources adapters, one function each. A factory builds nothing else, chooses nothing, and is called only
//! by an edge.

use asymptote_values::domain::manifest::types::ProductRoot;

/// The disk adapter over the product at `root`, which every path it is given is relative to.
#[must_use]
pub fn disk(root: ProductRoot) -> impl crate::outbound::sources::interface::SourcesPort {
    super::adapters::disk::adapter::SourcesDiskAdapter::new(root)
}
