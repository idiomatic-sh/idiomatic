//! Builds the packages adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

/// The path adapter: packages in local directories, and the packages the run made, held in memory.
#[must_use]
pub fn path() -> impl crate::outbound::packages::interface::PackagesPort {
    super::adapters::path::adapter::PackagesPathAdapter::default()
}
