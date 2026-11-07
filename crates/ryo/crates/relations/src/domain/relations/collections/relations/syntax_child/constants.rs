//! The names of the `syntax_child` relation: the text its name and its columns carry when they leave the engine.

use super::item::SyntaxChildColumn;
use crate::domain::relations::constants::{ORDINAL, PARENT};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "syntax-child"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "An edge of the syntax tree, ordered under its parent, with the child's role under it."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: SyntaxChildColumn) -> &'static str {
    match column {
        SyntaxChildColumn::Parent => PARENT,
        SyntaxChildColumn::Child => "child",
        SyntaxChildColumn::Ordinal => ORDINAL,
        SyntaxChildColumn::Field => "field",
        SyntaxChildColumn::File => "file",
    }
}
