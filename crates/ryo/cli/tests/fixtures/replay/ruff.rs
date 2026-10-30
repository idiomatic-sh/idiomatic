//! The copy of ruff a run is edited in.

/// Copies ruff to a directory of its own, where a store can be kept and a file can be edited.
///
/// # Panics
///
/// When the copy cannot be made.
#[must_use]
pub(crate) fn ruff_copy() -> tempfile::TempDir {
    copy_of(&super::base::snapshot::ruff())
}

/// Copies a directory to one of its own, where a store can be kept.
///
/// # Panics
///
/// When the copy cannot be made.
#[must_use]
pub(crate) fn copy_of(source: &std::path::Path) -> tempfile::TempDir {
    let copy = tempfile::tempdir().expect("a directory for the copy");
    let status = std::process::Command::new("cp")
        .arg("-a")
        .arg(format!("{}/.", source.display()))
        .arg(copy.path())
        .status()
        .expect("cp runs");
    assert!(status.success(), "ruff is copied");
    copy
}
