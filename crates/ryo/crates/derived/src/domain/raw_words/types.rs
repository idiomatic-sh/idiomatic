//! The types of the `raw_words` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::allocation::types::ElementCapacity;

/// One lowered key word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawWord(pub u64);

/// How many words make one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stride(pub usize);

/// A place in an ordering of rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderPlace(pub usize);

/// The places of the rows that share one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowRun {
    /// The first place of the run.
    pub start: OrderPlace,
    /// The place after the last of the run.
    pub end: OrderPlace,
}

/// Whether a row is the first of its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// The row is the first of its key.
    Yes,
    /// An earlier row has the key.
    No,
}

/// The capacity of each buffer of a raw ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderCapacities {
    /// The capacity of the lowered words.
    pub keys: ElementCapacity,
    /// The capacity of the ordering.
    pub order: ElementCapacity,
}
