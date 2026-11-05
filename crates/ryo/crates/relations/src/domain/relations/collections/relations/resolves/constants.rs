//! The names of the `resolves` relation: the text its name and its columns carry when they leave the engine.

use super::item::ResolvesColumn;
use crate::domain::relations::constants::OCCURRENCE;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "resolves"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A type name written in a type position, with the declaration or import that binds it in the file."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: ResolvesColumn) -> &'static str {
    match column {
        ResolvesColumn::Occurrence => OCCURRENCE,
        ResolvesColumn::Owner => "owner",
        ResolvesColumn::Name => "name",
        ResolvesColumn::Definition => "definition",
        ResolvesColumn::File => "file",
        ResolvesColumn::Span => "span",
    }
}
