//! Builds the workspace adapters from dependencies chosen by the composition.
use super::adapters::git::adapter::WorkspaceGitAdapter;
/// Construct the git implementation of the workspace contract.
#[must_use]
pub fn git(cache: std::path::PathBuf) -> impl super::interface::WorkspacePort {
    WorkspaceGitAdapter::new(super::adapters::git::settings::WorkspaceGitSettings { cache })
}
