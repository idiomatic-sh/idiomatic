//! The vocabulary of the optional `format-parse` relation.
use super::item::FormatParseColumn;

#[must_use]
pub fn name() -> &'static str {
    "format-parse"
}

#[must_use]
pub fn summary() -> &'static str {
    "Optional parsed string-format outcome; parsed zero-field formats, malformed formats and dynamic unknown inputs are distinct."
}

#[must_use]
pub fn column_name(column: FormatParseColumn) -> &'static str {
    match column {
        FormatParseColumn::Id => "id",
        FormatParseColumn::Dialect => "dialect",
        FormatParseColumn::Representation => "representation",
        FormatParseColumn::Outcome => "outcome",
        FormatParseColumn::ErrorCategory => "error-category",
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
