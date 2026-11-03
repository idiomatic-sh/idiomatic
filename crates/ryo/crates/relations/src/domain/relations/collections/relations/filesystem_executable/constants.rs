//! The names of the `filesystem_executable` relation: the text its name and its columns carry when they leave the engine.

use super::item::FilesystemExecutableColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "filesystem.executable"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A file with a known input-edge POSIX executable-bit observation; unknown observations remain incomplete, and unsupported permission models have no row."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: FilesystemExecutableColumn) -> &'static str {
    match column {
        FilesystemExecutableColumn::Path => "path",
        FilesystemExecutableColumn::Executable => "executable",
    }
}
