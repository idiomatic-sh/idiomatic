//! The types of the `positions` concept: the rows of a lowered step, as the positions of the elements of the file each is made of, and
//! the room the lowered steps work in.

use ryo_relations::domain::relations::types::Value;

use crate::domain::lowering::types::RowWidth;
use crate::domain::operating::types::OperatorContext;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};

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

/// Whether a lowered step has made its rows for the file being run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Readiness {
    /// It has.
    Made,
    /// It has not yet.
    NotMade,
}

/// The positions a walk or a lookup found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FoundPositions(pub Vec<u32>);

/// The positions of the rows of a lowered step, row after row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Slots(pub Vec<u32>);

/// The numbers the key columns of rows are stored as, one row after another.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Keys(pub Vec<u64>);

/// Rows by their place, in the order of their keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyOrder(pub Vec<usize>);

/// Which rows a step keeps, by the row's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Keeps(pub Vec<bool>);

/// A run of places in a key order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OrderRun(pub std::ops::Range<usize>);

/// Mutable positions and keys owned only by the current rule invocation.
#[derive(Debug, Default)]
pub(crate) struct LoweredRoom {
    /// The rows of each step, for the file being run.
    pub made: Vec<Positions>,
    /// Whether the step has made its rows for the file being run.
    pub ready: Vec<Readiness>,
    /// The positions a walk or a lookup found.
    pub found: FoundPositions,
    /// The values of a row's columns.
    pub cells: Vec<Value>,
    /// The keys of the rows of one side, one after another.
    pub keys: Keys,
    /// The key of one row of the other side.
    pub probe: Keys,
    /// The rows of the keyed side, in the order of their keys.
    pub order: KeyOrder,
    /// Which rows a step keeps.
    pub keep: Keeps,
    /// For each step of the rule being run, the class that decided it, when the pass did.
    pub picked: Vec<Option<ClassPlace>>,
}

/// A class of the steps that select from the elements of a tree, by its place among the classes of the call's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClassPlace(pub usize);

/// The room and the context a lowered step works with.
#[derive(Debug)]
pub(crate) struct Work<'a, 'c> {
    /// What the lowered steps keep from file to file.
    pub room: &'a mut LoweredRoom,
    /// What the operators read and sort in.
    pub context: &'a mut OperatorContext<'c>,
}

/// The two steps an operator reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Sides {
    /// The left one.
    pub left: StepId,
    /// The right one.
    pub right: StepId,
}

/// The columns of each side that make a key.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyColumns<'a> {
    /// The columns of the left side.
    pub left: &'a [ColumnIndex],
    /// The columns of the right side.
    pub right: &'a [ColumnIndex],
}
