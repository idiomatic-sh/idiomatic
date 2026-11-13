//! The types of the `table_words` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl. A table is written as words and read back from them.

use ryo_relations::domain::relations::types::Bits;

/// Words a table was written as, borrowed.
#[derive(Clone, Copy, Debug)]
pub struct WordSlice<'a>(pub &'a [u64]);

/// Existing diagnostic and named-publication decoding have different refusal policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeMode {
    Permissive,
    Complete,
}

/// Whether a decode read every word of its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Completeness {
    /// Every table was read to its end.
    Complete,
    /// A table was cut, malformed or refused.
    Incomplete,
}

/// Whether the words are spelled as `pack()` writes them: minimal varints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordSpelling {
    /// Each word is minimal and the last is whole.
    Canonical,
    /// A word is cut, overflows or is spelled another way.
    Altered,
}

/// Whether a byte range holds exactly one whole table frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// One whole table frame, nothing more.
    Whole,
    /// Anything else.
    Broken,
}

/// Whether the shape metadata of a compact table is consistent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableShape {
    /// Cells, frames and row offsets agree.
    Rectangular,
    /// Some of them disagree.
    Ragged,
}

/// Admitted offsets into immutable compact backing. Source-name mappings were
/// bound before any query borrow; materialization never mutates symbols.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactTable {
    pub words: std::sync::Arc<crate::domain::stored_facts::types::KeptWords>,
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    pub frames: Vec<CompactFrame>,
    pub rows: crate::domain::rows::types::RowCount,
    pub cells: CellCount,
    /// Where each row starts, counted from the start of the frame its first cell is in (`CompactFrame::first_row`): four
    /// bytes a row, where a position in the whole backing takes eight.
    pub row_offsets: Vec<FrameOffset>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactFrame {
    pub start: PackedOffset,
    pub end: PackedOffset,
    pub cells: CellCount,
    pub base: ryo_relations::domain::relations::types::PlaceNumber,
    /// How many rows start before this frame: the first row that starts in it, when any does.
    pub first_row: crate::domain::rows::types::RowIndex,
}

/// One byte cursor supplies exact immutable frame offsets; no second packed copy.
pub(crate) struct CompactCursor<'a> {
    pub bytes: PackedSlice<'a>,
    pub offset: PackedOffset,
}

/// The immutable packed words a compact table reads from, shared with its other readers.
#[derive(Clone, Debug)]
pub struct CompactBacking(pub std::sync::Arc<crate::domain::stored_facts::types::KeptWords>);

/// Packed bytes, borrowed.
#[derive(Clone, Copy, Debug)]
pub struct PackedSlice<'a>(pub &'a [u8]);

/// A byte position in packed words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackedOffset(pub usize);

/// A byte position in packed words, counted from the start of one frame.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::table_words::types::FrameOffset;
/// assert!(FrameOffset(4) < FrameOffset(12));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameOffset(pub u32);

/// Cells in a compact table or one of its frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellCount(pub usize);

/// Columns a compact row read asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnCount(pub usize);

/// Exact packed size of words, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EncodedSize(pub usize);

/// The tables read from packed words, and whether every table was read to its end.
#[derive(Clone, Debug)]
pub(crate) struct DecodedTables {
    /// The rows of every table read.
    pub table: crate::domain::rows::types::Table,
    /// Whether the input was read to its end.
    pub completeness: Completeness,
}

/// A compact table admitted from packed words, with the names whose spelling the caller checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admitted {
    /// Offsets into the immutable backing.
    pub table: CompactTable,
    /// Names whose spelling the caller checks before use.
    pub names: Vec<ryo_relations::domain::relations::types::Name>,
}

/// The address of a storage block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageAddress(pub usize);

/// The element count of a storage block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageLength(pub usize);

/// One storage block: where it starts and how long it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoragePart {
    /// Where the block starts.
    pub address: StorageAddress,
    /// How many elements it holds.
    pub length: StorageLength,
}

/// The storage blocks one compact table borrows from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactStorage {
    /// The packed words.
    pub words: StoragePart,
    /// The row offsets.
    pub offsets: StoragePart,
}
impl Iterator for CompactCursor<'_> {
    type Item = Bits;
    fn next(&mut self) -> Option<Bits> {
        let (mut word, mut shift) = (0_u64, 0_u32);
        while let Some(byte) = self.bytes.0.get(self.offset.0) {
            self.offset.0 += 1;
            word |= u64::from(byte & 0x7f)
                .checked_shl(shift)
                .unwrap_or_default();
            if byte & 0x80 == 0 {
                return Some(Bits(word));
            }
            shift += 7;
        }
        None
    }
}
