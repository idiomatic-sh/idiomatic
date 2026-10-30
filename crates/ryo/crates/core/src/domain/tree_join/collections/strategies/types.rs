//! The types of the `strategies` collection: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The columns a left row finds the elements of a tree by: the one that holds the node, and the column of the elements it is equal to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NodeColumns {
    /// The column of the left row that holds the node.
    pub left: ColumnIndex,
    /// The column of the elements it is equal to.
    pub key: ColumnIndex,
}

/// What a join's left row finds its right rows by.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Finding {
    /// The left column holding a node, and the column of the elements it is equal to.
    Node(ColumnIndex, ColumnIndex),
    /// The left columns holding a file and a stretch of it: the elements that cover that stretch exactly.
    Stretch(ColumnIndex, ColumnIndex),
    /// The left column holding a stretch of the file being read: the elements that cover it exactly.
    Span(ColumnIndex),
}

/// Whether a strategy finds the right rows of a join for a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fitting {
    /// The strategy finds them.
    Fits,
    /// Another strategy finds them.
    Misfits,
}
