//! The plain data of one inventory verification.

/// Whether every pinned rule of the inventory passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryVerdict {
    /// Every pinned rule has executed passing evidence.
    Passed,
    /// At least one rule failed or is missing; the summary records which.
    Failed,
}
