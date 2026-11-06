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

/// Admitted offsets into immutable compact backing. Source-name mappings were
/// bound before any query borrow; materialization never mutates symbols.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactTable {
    pub words: std::sync::Arc<crate::domain::stored_facts::types::KeptWords>,
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    pub frames: Vec<CompactFrame>,
    pub rows: usize,
    pub cells: usize,
    pub row_offsets: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactFrame {
    pub bytes: std::ops::Range<usize>,
    pub cells: usize,
    pub base: ryo_relations::domain::relations::types::PlaceNumber,
}

/// One byte cursor supplies exact immutable frame offsets; no second packed copy.
pub struct CompactCursor<'a> {
    pub bytes: &'a [u8],
    pub offset: usize,
}
impl Iterator for CompactCursor<'_> {
    type Item = Bits;
    fn next(&mut self) -> Option<Bits> {
        let (mut word, mut shift) = (0_u64, 0_u32);
        while let Some(byte) = self.bytes.get(self.offset) {
            self.offset += 1;
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
