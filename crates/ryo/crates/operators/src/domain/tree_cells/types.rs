//! The types of the `tree_cells` concept: the rows of a tree. Plain data, public fields and derives, with no constructor and no
//! inherent impl.

use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::FileId;

/// The rows of the tree of one file, as `syntax-child` or `syntax-tree` says them: in the order of the
/// elements. The table of the file says them already, so nothing is stored, and a lookup by parent or
/// by child is a read of the table.
#[derive(Clone, Copy, Debug)]
pub struct TreeRows<'a> {
    /// Which rows of the tree these are.
    pub form: TreeForm,
    /// The file's elements.
    pub elements: &'a Elements,
    /// The file.
    pub file: FileId,
}

/// Which relation the rows of a tree stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeForm {
    /// `syntax-child`: a row for each element but the root.
    Edges,
    /// `syntax-tree`: a row for each element.
    Elements,
}
