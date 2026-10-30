//! The names of the `string_literal` relation: the text its name and its columns carry when they leave the engine.

use super::item::StringLiteralColumn;
use crate::domain::relations::constants::{IMPORT, OCCURRENCE};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "string-literal"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A string literal, decoded."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: StringLiteralColumn) -> &'static str {
    match column {
        StringLiteralColumn::Occurrence => OCCURRENCE,
        StringLiteralColumn::Owner => "owner",
        StringLiteralColumn::Value => "value",
        StringLiteralColumn::File => "file",
        StringLiteralColumn::Span => "span",
        StringLiteralColumn::Import => IMPORT,
    }
}
