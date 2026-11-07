//! The names of the `scope` relation: the text its name and its columns carry when they leave the engine.

use super::item::ScopeColumn;
use crate::domain::relations::constants::PARENT;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "scope"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A scope of a file and the scope that directly encloses it; the module of the file encloses itself."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: ScopeColumn) -> &'static str {
    match column {
        ScopeColumn::Parent => PARENT,
        ScopeColumn::Child => "child",
        ScopeColumn::File => "file",
        ScopeColumn::Span => "span",
    }
}
