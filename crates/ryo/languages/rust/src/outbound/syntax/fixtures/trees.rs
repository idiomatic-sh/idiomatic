//! The trees of the sources the tests of the domain read, written by hand in the language's kinds, so those tests depend on no parser.
//! The adapter of the port is held to them: its test makes the table of each source and compares it element by element.

use super::script::Piece;

mod bindings;
mod control;
mod expressions;
mod items;
mod modules;
mod signatures;
mod uses;

/// The tree of `source`; none when no tree is written for it.
#[must_use]
pub(crate) fn tree_of(source: &str) -> Option<Piece> {
    signatures::tree_of(source)
        .or_else(|| expressions::tree_of(source))
        .or_else(|| modules::tree_of(source))
        .or_else(|| control::tree_of(source))
        .or_else(|| uses::tree_of(source))
        .or_else(|| items::tree_of(source))
        .or_else(|| bindings::tree_of(source))
}
