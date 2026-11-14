//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language's readers work out of a file once, on the first read that asks.

use crate::domain::bindings::types::BindingIndex;
use ryo::languages::ShapeIndex;
use std::cell::RefCell;

/// The indexes the readers of a file build on demand; each is emptied for the next file and keeps its capacity.
#[derive(Debug, Default)]
pub(crate) struct ReadIndexes {
    /// The names the file's declarations bind, by where each name is first declared.
    pub bindings: RefCell<BindingIndex>,
    /// The structural classes of the file's nodes, built on the first `expression_shape` read.
    pub shapes: ShapeIndex,
}
