//! The names of the `prose` relation: the text its name and its columns carry when they leave the engine.

use super::item::ProseColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "prose"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "The text a reader of a document reads, in runs, with its markup joined and its code masked."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: ProseColumn) -> &'static str {
    match column {
        ProseColumn::Id => "id",
        ProseColumn::Owner => "owner",
        ProseColumn::Text => "text",
        ProseColumn::File => "file",
        ProseColumn::Span => "span",
    }
}
