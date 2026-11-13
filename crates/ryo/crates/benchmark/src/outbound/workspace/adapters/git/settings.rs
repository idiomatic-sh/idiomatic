//! The storage root selected by the composition for owned checkouts.
/// Where this Git workspace adapter stores its isolated project snapshots.
#[derive(Debug)]
pub(crate) struct WorkspaceGitSettings {
    /// Root directory exclusively used by this benchmark's checkouts.
    pub cache: std::path::PathBuf,
}
