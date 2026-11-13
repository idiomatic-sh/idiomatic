//! The vocabulary of the optional `format-parse` relation.
use super::item::FormatParseColumn;
use crate::domain::relations::constants::{DIALECT, ERROR_CATEGORY, OUTCOME};

/// The relation's catalog name.
#[must_use]
pub fn name() -> &'static str {
    "format-parse"
}

/// A concise description shown with the relation's schema.
#[must_use]
pub fn summary() -> &'static str {
    "Optional parsed string-format outcome; parsed zero-field formats, malformed formats and dynamic unknown inputs are distinct."
}

/// The stable RyoQL name for a declared column.
#[must_use]
pub fn column_name(column: FormatParseColumn) -> &'static str {
    match column {
        FormatParseColumn::Id => "id",
        FormatParseColumn::Dialect => DIALECT,
        FormatParseColumn::Representation => "representation",
        FormatParseColumn::Outcome => OUTCOME,
        FormatParseColumn::ErrorCategory => ERROR_CATEGORY,
        FormatParseColumn::ErrorCharacter => "error-character",
        FormatParseColumn::ErrorIndex => "error-index",
        FormatParseColumn::PositionalConsumption => "positional-consumption",
        FormatParseColumn::MappingPresent => "mapping-present",
        FormatParseColumn::DynamicWidth => "dynamic-width",
        FormatParseColumn::DynamicPrecision => "dynamic-precision",
        FormatParseColumn::File => "file",
        FormatParseColumn::Span => "span",
    }
}
