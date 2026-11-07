//! The types of the `positions` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The position of one row in a shared list of positions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowPosition(pub u32);

/// Whether one element is marked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// The element is a member.
    Member,
    /// The element is not a member.
    Absent,
}
