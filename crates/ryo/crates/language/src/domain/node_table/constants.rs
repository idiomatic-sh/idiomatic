//! The constants of the `node_table` concept.

/// How many kinds one presence word holds.
pub(crate) const PRESENCE_BITS: usize = u64::BITS as usize;

/// The mark of a token among the open nodes' children written so far: its place among the pending tokens follows it, as its
/// id is given only when its segment closes.
pub(crate) const PENDING: u32 = 1 << 31;
