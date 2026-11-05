//! The names of the `mapping` relation: the text its name and its columns carry when they leave the engine.

use super::item::MappingColumn;
use crate::domain::relations::constants::ORDINAL;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "mapping"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A mapping of a data document: a table of keys, however it is written."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: MappingColumn) -> &'static str {
    match column {
        MappingColumn::Id => "id",
        MappingColumn::Kind => "kind",
        MappingColumn::Ordinal => ORDINAL,
        MappingColumn::Owner => "owner",
        MappingColumn::File => "file",
    }
}
