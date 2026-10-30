//! One collection: each relation the language reads from a file's tree, in order.

pub(crate) mod comment;
pub(crate) mod entry;
pub(crate) mod mapping;
pub(crate) mod sequence_element;
pub(crate) mod string_literal;
pub(crate) mod syntax_child;
pub(crate) mod syntax_error;
pub(crate) mod syntax_tree;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
