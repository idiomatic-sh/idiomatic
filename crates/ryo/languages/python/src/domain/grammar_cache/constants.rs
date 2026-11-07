//! The names and bounds of this concept.

/// The text ``.
pub(crate) const EMPTY: &str = "";

/// Entries one content memo may hold for the following source.
pub(crate) const MEMO_ENTRIES: usize = 64;

/// Longest input text, in bytes of capacity, one memo entry may keep as its key.
pub(crate) const KEY_CAPACITY: usize = 4096;

/// Largest assembly buffer, in bytes of capacity, a memo keeps for reuse.
pub(crate) const SCRATCH_CAPACITY: usize = 2048;

/// Most elements of capacity an active vector keeps across a reset.
pub(crate) const VECTOR_CAPACITY: usize = 256;

/// Most entries of capacity an ordered lookup keeps across a reset.
pub(crate) const MAP_CAPACITY: usize = 64;
