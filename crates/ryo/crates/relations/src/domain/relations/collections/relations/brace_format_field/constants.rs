//! Names of the `brace-format-field` columns.
use super::item::BraceFormatFieldColumn;
use crate::domain::relations::constants::{CONTEXT, ORDINAL};

/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "brace-format-field"
}

/// Ordered selector fields in decoded brace-format strings.
#[must_use]
pub fn summary() -> &'static str {
    "Optional field selectors from decoded brace-format strings, including nested format-spec fields and duplicate occurrences."
}

/// A column's RyoQL spelling.
#[must_use]
pub fn column_name(column: BraceFormatFieldColumn) -> &'static str {
    match column {
        BraceFormatFieldColumn::Id => "id",
        BraceFormatFieldColumn::Ordinal => ORDINAL,
        BraceFormatFieldColumn::Context => CONTEXT,
        BraceFormatFieldColumn::ParentOrdinal => "parent-ordinal",
        BraceFormatFieldColumn::SelectorKind => "selector-kind",
        BraceFormatFieldColumn::SelectorIndex => "selector-index",
        BraceFormatFieldColumn::IndexOverflow => "index-overflow",
        BraceFormatFieldColumn::SelectorName => "selector-name",
        BraceFormatFieldColumn::File => "file",
        BraceFormatFieldColumn::Span => "span",
    }
}
