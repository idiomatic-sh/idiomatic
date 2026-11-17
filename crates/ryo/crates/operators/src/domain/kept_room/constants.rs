//! The cap on the room a worker's scratch keeps past the end of a file.

/// The most room, in items, a worker's scratch buffer keeps past the end of a file: 4 Ki, 64 KiB of 16-byte cells.
pub(crate) const KEPT_ROOM: usize = 1 << 12;
