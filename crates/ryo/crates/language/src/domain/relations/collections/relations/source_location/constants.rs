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
    "The one-based start and exclusive-end lines of every syntax node and token."
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
    }
}
