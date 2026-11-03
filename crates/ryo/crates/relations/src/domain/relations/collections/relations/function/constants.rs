//! The names of the `function` relation: the text its name and its columns carry when they leave the engine.

use super::item::FunctionColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "function"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A function declaration."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: FunctionColumn) -> &'static str {
    match column {
        FunctionColumn::Id => "id",
        FunctionColumn::Name => "name",
        FunctionColumn::File => "file",
        FunctionColumn::Span => "span",
    }
}
