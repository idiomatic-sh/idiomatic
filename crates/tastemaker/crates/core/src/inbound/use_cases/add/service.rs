//! The `add` service: local free-function helpers, private to this use case.

use crate::domain::paths::service::join;
use crate::domain::paths::types::{PathText, RepoPath};
use crate::outbound::repository::interface::FoundRepository;

/// Where the entry applies, `at` given from where the command ran: relative to the workspace's directory, as every path
/// of its file is, and the workspace's own directory written `.`, since an empty include reaches nothing.
pub(super) fn entry_path(found: &dyn FoundRepository, at: PathText<'_>) -> RepoPath {
    let at = join(found.start(), at).unwrap_or_default();
    let at = if found.workspace().0.is_empty() {
        at
    } else if at == found.workspace().clone() {
        RepoPath::default()
    } else {
        RepoPath(
            at.0.strip_prefix(&*format!("{}/", found.workspace().0))
                .map_or_else(|| at.0.clone(), Into::into),
        )
    };
    if at.0.is_empty() {
        RepoPath(crate::domain::paths::constants::CURRENT.into())
    } else {
        at
    }
}
