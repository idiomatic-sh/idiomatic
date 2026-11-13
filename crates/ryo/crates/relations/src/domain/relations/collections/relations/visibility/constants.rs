//! The names of the `visibility` relation: the text its name and its columns carry when they leave the engine.

use super::item::VisibilityColumn;
use crate::domain::relations::constants::OCCURRENCE;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "visibility"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "The visibility a declaration is written with, as written."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: VisibilityColumn) -> &'static str {
    match column {
        VisibilityColumn::Occurrence => OCCURRENCE,
        VisibilityColumn::Declaration => crate::domain::relations::constants::DECLARATION,
        VisibilityColumn::Text => "text",
        VisibilityColumn::File => "file",
        VisibilityColumn::Span => "span",
    }
}
