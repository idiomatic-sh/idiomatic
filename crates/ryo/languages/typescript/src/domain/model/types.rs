//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language works out of a file once, on a reader's first lookup.

use crate::domain::bindings::types::BindingIndex;
use std::cell::RefCell;

/// The indexes the readers of one file share, each worked out on its first lookup and cleared before the next file.
#[derive(Debug, Default)]
pub(crate) struct ReadIndexes {
    /// The names the file's declarations bind, by where each name is first declared.
    pub bindings: RefCell<BindingIndex>,
}
