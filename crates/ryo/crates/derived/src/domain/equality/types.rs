//! The types of the `equality` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::key::types::{ChainLink, HashedRow, KeyCellsHash};
use ryo_values::domain::rows::types::RowIndex;

/// A hash-chain index of the keys of finite rows.
#[derive(Debug, Default)]
pub struct KeyIndex {
    /// The indexed rows in physical order.
    pub rows: Vec<RowIndex>,
    /// The hash of each row's key.
    pub hashes: Vec<KeyCellsHash>,
    /// The first link of each bucket.
    pub heads: Vec<ChainLink>,
    /// The next link after each row.
    pub next: Vec<ChainLink>,
    /// The rows in key order.
    pub sorted: Vec<RowIndex>,
    /// The rows with their hashes.
    pub pairs: Vec<HashedRow>,
    /// The distinct key hashes.
    pub runs: hashbrown::HashSet<KeyCellsHash>,
}

/// The capacity of each buffer of a key index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyIndexCapacities {
    /// The capacity of the rows.
    pub rows: ElementCapacity,
    /// The capacity of the hashes.
    pub hashes: ElementCapacity,
    /// The capacity of the heads.
    pub heads: ElementCapacity,
    /// The capacity of the links.
    pub links: ElementCapacity,
    /// The capacity of the sorted rows.
    pub sorted: ElementCapacity,
    /// The capacity of the pairs.
    pub pairs: ElementCapacity,
    /// The capacity of the distinct hashes.
    pub runs: ElementCapacity,
}
