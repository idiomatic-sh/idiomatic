//! The names of the `filesystem_file` relation: the text its name and its columns carry when they leave the engine.

use super::item::FilesystemFileColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "filesystem.file"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A file of the repository, whatever kind of file it is; the engine gives it, no language does."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: FilesystemFileColumn) -> &'static str {
    match column {
        FilesystemFileColumn::Path => "path",
    }
}
