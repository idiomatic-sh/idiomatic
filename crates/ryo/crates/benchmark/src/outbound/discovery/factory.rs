//! Builds the discovery adapters from the command the composition chose.
use super::adapters::listing::adapter::DiscoveryListingAdapter;
use std::{ffi::OsString, path::PathBuf};
/// Construct the adapter that asks a tool's own command which files it selects in a worktree.
#[must_use]
pub fn listing(
    executable: PathBuf,
    arguments: Vec<OsString>,
) -> impl super::interface::DiscoveryPort {
    DiscoveryListingAdapter::new(executable, arguments)
}
