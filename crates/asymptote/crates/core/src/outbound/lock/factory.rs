//! Builds the lock adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

use crate::domain::manifest::types::ProductRoot;

/// The TOML adapter over the lock of the product at `root`: one file per operation under `.asymptote/operations`.
#[must_use]
pub fn toml(root: ProductRoot) -> impl crate::outbound::lock::interface::LockPort {
    super::adapters::toml::adapter::LockTomlAdapter::new(root)
}
