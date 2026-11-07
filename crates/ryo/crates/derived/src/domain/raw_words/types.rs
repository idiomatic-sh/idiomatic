//! The types of the `raw_words` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::allocation::types::ElementCapacity;

/// Borrowed lowered key words: the bare words inside are named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawWords<'a>(pub &'a [u64]);

/// Owned lowered key words: the bare words inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawKeys(pub Vec<u64>);

/// How many words make one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyStride(pub usize);

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

/// For each place of an ordering, whether the row there is the first of its key: the bare flags inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeepFlags(pub Vec<bool>);

/// The capacity of each buffer of a raw ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderCapacities {
    /// The capacity of the lowered words.
    pub keys: ElementCapacity,
    /// The capacity of the ordering.
    pub order: ElementCapacity,
}
