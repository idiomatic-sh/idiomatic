//! The types of the `attribute_paths` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::{Position, Tree};

/// A path an attribute writes: the element it starts at and the element it ends at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WrittenPath {
    /// The path node of a meta, or the first token (a segment or the first `:` of a leading `::`) of a path written in a token tree.
    pub(crate) first: Position,
    /// The path node itself, or the last segment token of a path written in a token tree.
    pub(crate) last: Position,
}

/// The paths one element writes, in the order the file writes them: the one path of a meta's path node, or each path among the
/// tokens directly in a token tree.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WrittenPaths<'a> {
    /// The file's tree.
    pub(crate) tree: &'a Tree<'a>,
    /// The path of a meta's path node, until it is given.
    pub(crate) single: Option<WrittenPath>,
    /// The child of the token tree the next path is looked for from, none when every child is read.
    pub(crate) cursor: Option<Position>,
    /// The last element inside the token tree.
    pub(crate) end: Position,
}
