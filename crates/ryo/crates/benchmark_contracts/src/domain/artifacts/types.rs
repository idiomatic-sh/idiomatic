//! Canonical artifact values.
pub mod digest;
pub mod relative_artifact_path;

/// Observed execute-bit state, independent of a raw transport boolean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileExecutable {
    /// At least one POSIX execute bit is set.
    Set,
    /// No POSIX execute bit is set.
    Unset,
}

/// One borrowed observation in the canonical, strictly ordered metadata inventory.
#[derive(Clone, Copy, Debug)]
pub struct FileMetadataEntry<'a> {
    /// Exact declared source identity.
    pub path: &'a ryo::linter::RelativePath,
    /// Observed executable state.
    pub executable: FileExecutable,
}
