//! The types of the `table_cells` concept: the rows of a tree and the cells read from them. Plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::FileId;

/// A place in a table: the number of a row.

/// The rows of the tree of one file, as `syntax-child` or `syntax-tree` says them: in the order of the
/// elements. The table of the file says them already, so nothing is stored, and a lookup by parent or
/// by child is a read of the table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeRows<'a> {
    /// Which rows of the tree these are.
    pub form: TreeForm,
    /// The file's elements.
    pub elements: &'a Elements,
    /// The file.
    pub file: FileId,
}

/// Which relation the rows of a tree stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TreeForm {
    /// `syntax-child`: a row for each element but the root.
    Edges,
    /// `syntax-tree`: a row for each element.
    Elements,
}

/// The places of the columns of a row that comparisons read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColumnPlaces(pub Vec<usize>);

/// Whether the cells asked for were read from a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellsRead {
    /// The row is there and the cells were read.
    Read,
    /// The row or a column is not there.
    Missing,
}
