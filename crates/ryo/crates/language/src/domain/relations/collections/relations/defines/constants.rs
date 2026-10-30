//! The names of the `defines` relation: the text its name and its columns carry when they leave the engine.

use super::item::DefinesColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "defines"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A name a file declares or brings in, under its shared kind, with the language's own refinement."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: DefinesColumn) -> &'static str {
    match column {
        DefinesColumn::Id => "id",
        DefinesColumn::Kind => "kind",
        DefinesColumn::Refinement => "refinement",
        DefinesColumn::Name => "name",
        DefinesColumn::Owner => "owner",
        DefinesColumn::File => "file",
        DefinesColumn::Span => "span",
    }
}
