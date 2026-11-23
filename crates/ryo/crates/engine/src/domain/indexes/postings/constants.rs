//! Bounds on the allocation space retained between files.
/// Maximum retained capacity for the rows and offsets of earlier indexes.
pub(crate) const INDEX_CELLS: usize = 1 << 20;
/// Largest key scratch retained after a build.
pub(crate) const KEY_CELLS: usize = 1 << 16;
