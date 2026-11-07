//! The types of the `key` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// One cell reduced to a word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HashWord(pub u64);

/// The hash of the cells of one key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyCellsHash(pub u64);

/// A link of a hash chain: the place after the linked row, zero when the chain ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChainLink(pub u32);

/// A row and the hash of its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HashedRow {
    /// The hash of the row's key.
    pub hash: KeyCellsHash,
    /// The row.
    pub row: RowIndex,
}

/// Key-column vocabulary shared by immutable probes and derived builders.
#[derive(Clone, Copy, Debug)]
pub enum On<'a> {
    /// Every cell of the row.
    Whole,
    /// The cells of these columns.
    Columns(&'a [ColumnIndex]),
}
