//! The names of the `entry` relation: the text its name and its columns carry when they leave the engine.

use super::item::EntryColumn;
use crate::domain::relations::constants::{
    BOOLEAN, DATETIME, FLOAT, INTEGER, OCCURRENCE, ORDINAL, RADIX,
};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "entry"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A key of a data document with its value."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: EntryColumn) -> &'static str {
    match column {
        EntryColumn::Decoded => "decoded",
        EntryColumn::Key => "key",
        EntryColumn::Kind => "kind",
        EntryColumn::Occurrence => OCCURRENCE,
        EntryColumn::Ordinal => ORDINAL,
        EntryColumn::Owner => "owner",
        EntryColumn::File => "file",
        EntryColumn::Span => "span",
        EntryColumn::Text => "text",
        EntryColumn::Boolean => BOOLEAN,
        EntryColumn::Integer => INTEGER,
        EntryColumn::Float => FLOAT,
        EntryColumn::Datetime => DATETIME,
        EntryColumn::Radix => RADIX,
    }
}
