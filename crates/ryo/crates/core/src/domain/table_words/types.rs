//! The types of the `table_words` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl. A table is written as words and read back from them.

/// Words a table was written as, borrowed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WordSlice<'a>(pub &'a [u64]);

/// Existing diagnostic and named-publication decoding have different refusal policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DecodeMode {
    Permissive,
    Complete,
}

/// Admitted offsets into immutable compact backing. Source-name mappings were
/// bound before any query borrow; materialization never mutates symbols.
#[derive(Clone, Debug)]
pub(crate) struct CompactTable {
    pub(super) words: std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>,
    pub(super) kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    pub(super) frames: Vec<CompactFrame>,
    pub(super) rows: usize,
    pub(super) cells: usize,
}
#[derive(Clone, Debug)]
pub(super) struct CompactFrame {
    pub(super) bytes: std::ops::Range<usize>,
    pub(super) cells: usize,
    pub(super) base: ryo_relations::domain::relations::types::PlaceNumber,
}
