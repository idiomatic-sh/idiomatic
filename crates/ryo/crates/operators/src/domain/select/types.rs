//! Select's predicate outcome and its answer.

/// What the already evaluated predicate decided about the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PredicateOutcome {
    /// The predicate holds: the row is kept.
    Holds,
    /// The predicate fails: the row is dropped.
    Fails,
}

/// Whether the row was kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowSelection {
    /// The row is kept.
    Kept,
    /// The row is dropped.
    Dropped,
}
