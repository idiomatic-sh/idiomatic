//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language's readers work out of a file once, on the first read that asks.

use ryo::languages::ShapeIndex;

/// The indexes the readers of a file build on demand; each is emptied for the next file and keeps its capacity.
#[derive(Debug, Default)]
pub(crate) struct ReadIndexes {
    /// The structural classes of the file's nodes, built on the first `expression_shape` read.
    pub shapes: ShapeIndex,
}
