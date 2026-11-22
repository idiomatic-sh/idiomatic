//! The types of the `keying` part: where a key's bit is, and a row's key and number in one word.

use ryo_relations::domain::relations::types::Count;

/// Where a key's bit is: its word of `present`, and the bit in it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BitAt {
    /// The word.
    pub(crate) word: Count,
    /// The bit.
    pub(crate) bit: KeyBits,
}

/// A row's key number above its row number, in one word: sorted, the rows are in key order and, within a key, in the order
/// they were written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KeyedRow(pub(crate) u64);

/// Sixty-four keys of an index, one bit each: the key has a row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyBits(pub(crate) u64);
