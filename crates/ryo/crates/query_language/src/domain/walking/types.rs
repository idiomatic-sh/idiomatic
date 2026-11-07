//! The types of the `walking` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// How many columns the rows of an input have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputWidth(pub u32);

/// The row a fused lookup starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StartingRow {
    /// How many columns it has.
    pub width: InputWidth,
    /// The column holding the node it starts from.
    pub start: ColumnIndex,
    /// The edge column that is the element reached.
    pub found: ColumnIndex,
}

/// What a list of comparisons asks of the elements of the syntax tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElementDemand {
    /// Only that an element be a node.
    OnlyNodes,
    /// Anything else.
    More,
}
