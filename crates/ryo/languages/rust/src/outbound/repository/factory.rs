//! Builds the repository adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use ryo::languages::RootPath;

/// The `native` repository adapter: the files of the machine below `root`, the root of the run's repository.
#[must_use]
pub(crate) fn native(
    root: RootPath,
) -> impl crate::outbound::repository::interface::RepositoryPort {
    super::adapters::native::adapter::RepositoryNativeAdapter::new(root)
}
