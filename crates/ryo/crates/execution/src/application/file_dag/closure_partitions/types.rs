//! The values a partition cursor keeps.

/// The next position of the shared ordering a cursor reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct OrderCursor(pub usize);
