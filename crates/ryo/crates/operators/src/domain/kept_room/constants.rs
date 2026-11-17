//! The caps on the room a worker's scratch keeps past the end of a file, and on how many emptied buffers its pools keep.

/// The most room, in items, a worker's scratch buffer keeps past the end of a file: 4 Ki, 64 KiB of 16-byte cells.
pub(crate) const KEPT_ROOM: usize = 1 << 12;

/// The most emptied keyed-row tables a worker's pool keeps for its next indexes: the shared indexes of a plan and its streamed
/// stages' own indexes and repeats take their room from it and give it back, so a plan of many indexes keeps no more than a
/// plan of a few.
pub(crate) const POOL_INDEXES: usize = 64;

/// The most emptied node tables a worker's pool keeps for its next nodes.
pub(crate) const POOL_TABLES: usize = 256;

/// A node table of more room for kinds than this is dropped, not pooled.
pub(crate) const POOL_TABLE_KINDS: usize = 64;
