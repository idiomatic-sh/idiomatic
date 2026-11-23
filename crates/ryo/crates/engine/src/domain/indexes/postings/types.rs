//! Ordered postings, their lookup directory and reusable worker space.
use rustc_hash::FxHashMap;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::rc::Rc;

/// Rows sharing a key are consecutive and in input order.
#[derive(Debug, Default)]
pub struct PostingIndex {
    /// Offsets when the key space is small enough to address directly.
    pub offsets: PackedPositions,
    /// Ranges for sparse keys, including hashes of composite keys.
    pub ranges: FxHashMap<CellWord, PostingInterval>,
    /// Original row numbers.
    pub rows: PackedPositions,
}

/// Persistent allocation space for key construction and posting directories.
#[derive(Debug, Default)]
pub struct PostingBuffers {
    /// Uniquely owned index headers waiting for the next file.
    pub shared: Vec<Rc<PostingIndex>>,
    /// Posting row and offset vectors.
    pub indexes: Vec<PackedPositions>,
    /// Combined capacity of posting vectors.
    pub index_capacity: Count,
    /// Cleared sparse lookup directories.
    pub directories: Vec<FxHashMap<CellWord, PostingInterval>>,
    /// One computed key per input row.
    pub keys: Vec<CellWord>,
    /// Write cursor per dense key.
    pub cursor: PackedPositions,
    /// Key and row pairs for sparse sorting.
    pub order: Vec<SortedPosting>,
}

/// A compact physical row position or end offset in a posting array.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PostingPosition(pub u32);

/// A run of equal keys in a sparse posting array.
#[derive(Clone, Copy, Debug)]
pub struct PostingInterval {
    /// First matching row in the array.
    pub start: PostingPosition,
    /// First position after the matching rows.
    pub end: PostingPosition,
}

/// A key and its original row, sorted lexicographically to give stable postings.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SortedPosting {
    /// Value of the probe key.
    pub key: CellWord,
    /// Original row, breaking ties in input order.
    pub row: PostingPosition,
}

/// Contiguous native ordinals shared with the file's row-selection kernels.
#[derive(Debug, Default)]
pub struct PackedPositions(pub Vec<u32>);

/// A borrowed posting in ascending source-row order.
#[derive(Clone, Copy, Debug)]
pub struct PostingRows<'a>(pub &'a [u32]);

/// A grouping key. Short tuples remain inline and hash exactly like their slice.
pub type PartitionKey<Word> = smallvec::SmallVec<[Word; 4]>;

/// An ordered group with one slot held inline.
pub type KeyGroup<Item> = smallvec::SmallVec<[Item; 1]>;

/// Plan-bound column ordinals in comparison order; borrowed for the index build.
#[derive(Clone, Copy, Debug)]
pub struct PostingColumns<'a>(pub &'a [usize]);
