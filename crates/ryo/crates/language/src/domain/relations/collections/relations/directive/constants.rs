//! The names of the `directive` relation: the text its name and its columns carry when they leave the engine.

use super::item::DirectiveColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "directive"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "An instruction to the build tool written in source, by its kind and its text as the tool reads it."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: DirectiveColumn) -> &'static str {
    match column {
        DirectiveColumn::File => "file",
        DirectiveColumn::Span => "span",
        DirectiveColumn::Kind => "kind",
        DirectiveColumn::Text => "text",
    }
}
