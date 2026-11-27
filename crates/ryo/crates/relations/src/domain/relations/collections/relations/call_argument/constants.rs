//! The names of the `call_argument` relation: the text its name and its columns carry when they leave the engine.

use super::item::CallArgumentColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "call-argument"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "An argument of a call of a path written out, by its place among the arguments."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: CallArgumentColumn) -> &'static str {
    match column {
        CallArgumentColumn::Call => "call",
        CallArgumentColumn::Position => "position",
        CallArgumentColumn::Expression => "expression",
        CallArgumentColumn::Passes => "passes",
        CallArgumentColumn::File => "file",
        CallArgumentColumn::Span => "span",
    }
}
