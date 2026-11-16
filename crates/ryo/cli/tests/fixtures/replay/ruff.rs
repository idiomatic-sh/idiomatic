//! The copy of ruff a run is edited in.

/// Copies ruff to a directory of its own, where a store can be kept and a file can be edited.
///
/// # Panics
///
/// When the copy cannot be made.
#[must_use]
pub(crate) fn ruff_copy() -> tempfile::TempDir {
    super::copy::copy_of(&super::snapshot::ruff())
}
