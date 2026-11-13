//! The names of the `filesystem_line_count` relation: the text its name and its columns carry when they leave the engine.

use super::item::FilesystemLineCountColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "filesystem.line-count"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A file of the repository with its total line count, whatever kind of file it is; the engine gives it, no language does."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: FilesystemLineCountColumn) -> &'static str {
    match column {
        FilesystemLineCountColumn::Path => "path",
        FilesystemLineCountColumn::Lines => "lines",
    }
}
