//! Names of the `brace-format-parse` columns.
use super::item::BraceFormatParseColumn;

/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "brace-format-parse"
}

/// Parsed status and aggregate selector counts for decoded brace-format strings.
#[must_use]
pub fn summary() -> &'static str {
    "Optional outcomes for decoded brace-format strings, distinguishing valid, malformed, dynamic, and unavailable inputs."
}

/// A column's RyoQL spelling.
#[must_use]
pub fn column_name(column: BraceFormatParseColumn) -> &'static str {
    match column {
        BraceFormatParseColumn::Id => "id",
        BraceFormatParseColumn::Representation => "representation",
        BraceFormatParseColumn::Outcome => "outcome",
        BraceFormatParseColumn::ErrorCategory => "error-category",
        BraceFormatParseColumn::AutomaticCount => "automatic-count",
        BraceFormatParseColumn::IndexedCount => "indexed-count",
        BraceFormatParseColumn::KeywordCount => "keyword-count",
        BraceFormatParseColumn::NestedFieldCount => "nested-field-count",
        BraceFormatParseColumn::File => "file",
        BraceFormatParseColumn::Span => "span",
    }
}
