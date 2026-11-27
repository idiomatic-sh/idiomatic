//! Names of the source-location relation and its columns.

use super::item::SourceLocationColumn;

/// The query name.
#[must_use]
pub fn name() -> &'static str {
    "source-location"
}

/// What the relation contains.
#[must_use]
pub fn summary() -> &'static str {
    "One-based lines, zero-based UTF-8 byte offsets and byte columns for syntax nodes and tokens."
}

/// The query name of a column.
#[must_use]
pub fn column_name(column: SourceLocationColumn) -> &'static str {
    match column {
        SourceLocationColumn::Id => "id",
        SourceLocationColumn::File => "file",
        SourceLocationColumn::Span => "span",
        SourceLocationColumn::StartLine => "start-line",
        SourceLocationColumn::EndLine => "end-line",
        SourceLocationColumn::StartByte => "start-byte",
        SourceLocationColumn::EndByte => "end-byte",
        SourceLocationColumn::StartColumn => "start-column",
        SourceLocationColumn::EndColumn => "end-column",
    }
}
