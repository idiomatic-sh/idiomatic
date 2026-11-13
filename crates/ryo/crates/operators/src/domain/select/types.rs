//! Select's predicate outcome and its answer.

/// Whether the row was kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowSelection {
    /// The row is kept.
    Kept,
    /// The row is dropped.
    Dropped,
}
