//! The types of the `keyed_ranges` concept: a worker's kept answers by slot (a probed step and the column it is probed on)
//! and key.

/// A slot's number in its store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SlotId(pub u32);

/// One key's rows: where they start in the slot's columns and how many, kept for the file whose generation it carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyRange {
    /// The first row (`PENDING` while the key's batch runs).
    pub(crate) start: u32,
    /// How many rows.
    pub(crate) length: u32,
    /// The file it was filed in (a dense entry of another file is absent).
    pub(crate) generation: u32,
    /// The last gather that read it (a key asked twice in one list is read once).
    pub(crate) stamp: u32,
}

/// One slot: its rows as columns, and every key's range.
#[derive(Debug, Default)]
pub struct RangeSlot {
    /// The rows, one buffer a column.
    pub(crate) columns: Vec<Vec<u64>>,
    /// The ranges of keys below the file's dense bound, by key.
    pub(crate) dense: Vec<KeyRange>,
    /// The ranges of every other key.
    pub(crate) sparse: hashbrown::HashMap<u64, KeyRange>,
    /// Whether the slot was used in this file.
    pub(crate) touched: bool,
    /// Whether this file's keys below the dense bound are kept by position (the slot asked for many keys of the last file
    /// it was used in), else in the flat hash.
    pub(crate) dense_mode: bool,
    /// How many keys were filed or asked in this file.
    pub(crate) keys: u32,
}

/// A worker's kept answers: emptied per file, never freed.
#[derive(Debug, Default)]
pub struct KeyedRanges {
    pub(crate) slots: Vec<RangeSlot>,
    /// The slots used in this file.
    pub(crate) touched: Vec<SlotId>,
    /// The file's generation (starts at 1: a default entry is absent).
    pub(crate) generation: u32,
    /// Keys at or above this are kept in the flat hash: the file's row or node count.
    pub(crate) dense_bound: u64,
    /// The dense bound of the last file.
    pub(crate) last_bound: u64,
    pub(crate) stamp: u32,
    /// The batch's rows by key, reused.
    pub(crate) order: Vec<(u64, u32)>,
}

/// One key of an asked list, as `ask` found it: its rows' range, or (start `PENDING`) the position in the asked list of a
/// key whose rows its batch is about to file. Each key of a list once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AskedRange {
    pub(crate) start: u32,
    pub(crate) length: u32,
}

/// What asking a slot for a list of keys found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Every key is filed: `gather` answers them.
    Filed,
    /// Some keys are not: they were written to the missing list and marked pending; run the step for them and `file` its rows.
    Missing,
}
