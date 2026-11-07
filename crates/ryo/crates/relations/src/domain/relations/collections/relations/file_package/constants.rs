//! The names of the `file_package` relation: the text its name and its columns carry when they leave the engine.

use super::item::FilePackageColumn;
use crate::domain::relations::constants::PACKAGE;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "file-package"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "The package a file of the repository belongs to."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: FilePackageColumn) -> &'static str {
    match column {
        FilePackageColumn::Path => "path",
        FilePackageColumn::Package => PACKAGE,
    }
}
