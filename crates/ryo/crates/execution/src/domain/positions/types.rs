//! The types of the `positions` concept: the rows of a lowered step, as the positions of the elements of the file each is made of, and
//! the room the lowered steps work in.

use crate::domain::lowering::types::RowWidth;

/// How many numbers stand for one row of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stride(pub usize);

/// The rows of a lowered step: the positions of the elements of the file each row is made of, one row after another.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Positions {
    /// How many positions a row holds.
    pub width: RowWidth,
    /// The positions, row after row.
    pub slots: Slots,
}

/// The positions of one row of a lowered step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowPositions<'a>(pub &'a [u32]);

/// The positions a walk or a lookup found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FoundPositions(pub Vec<u32>);

/// The positions of the rows of a lowered step, row after row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Slots(pub Vec<u32>);

/// The numbers the key columns of rows are stored as, one row after another.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Keys(pub Vec<u64>);

/// Which rows a step keeps, by the row's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Keeps(pub Vec<bool>);

/// A class of the steps that select from the elements of a tree, by its place among the classes of the call's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClassPlace(pub usize);
