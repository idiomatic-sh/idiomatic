//! The types of the `table_cells` concept: the rows of a tree and the cells read from them. Plain data, public fields and derives, with no constructor and no inherent impl.

/// The places of the columns of a row that comparisons read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnPlaces(pub Vec<usize>);

/// Whether the cells asked for were read from a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellsRead {
    /// The row is there and the cells were read.
    Read,
    /// The row or a column is not there.
    Missing,
}
