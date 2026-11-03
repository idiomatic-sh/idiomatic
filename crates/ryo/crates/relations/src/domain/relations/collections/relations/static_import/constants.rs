//! The names of the `static_import` relation: the text its name and its columns carry when they leave the engine.

use super::item::StaticImportColumn;
use crate::domain::relations::constants::{BINDING, OCCURRENCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "static-import"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A name an import brings into scope."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: StaticImportColumn) -> &'static str {
    match column {
        StaticImportColumn::Occurrence => OCCURRENCE,
        StaticImportColumn::Owner => "owner",
        StaticImportColumn::Binding => BINDING,
        StaticImportColumn::Target => TARGET,
        StaticImportColumn::File => "file",
        StaticImportColumn::Span => "span",
    }
}
