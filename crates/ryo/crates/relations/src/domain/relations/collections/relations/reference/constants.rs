//! The names of the `reference` relation: the text its name and its columns carry when they leave the engine.

use super::item::ReferenceColumn;
use crate::domain::relations::constants::{OCCURRENCE, REFERENCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    REFERENCE
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A name used where it is not declared."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: ReferenceColumn) -> &'static str {
    match column {
        ReferenceColumn::Occurrence => OCCURRENCE,
        ReferenceColumn::Owner => "owner",
        ReferenceColumn::Target => TARGET,
        ReferenceColumn::File => "file",
        ReferenceColumn::Span => "span",
    }
}
