//! Bounds on the reusable memory held by a worker.
/// Capacity classes retained for homogeneous result columns.
pub(crate) const CLASSES: usize = 20;
/// Upper bound on reusable capacity, measured in packed cells.
pub(crate) const COLUMN_CELLS: usize = 1 << 22;
/// Maximum number of empty headers retained by a single worker.
pub(crate) const HEADERS: usize = 1 << 14;
/// Maximum retained capacity of worker selection buffers.
pub(super) const SELECTION_ROWS: usize = 1 << 20;
