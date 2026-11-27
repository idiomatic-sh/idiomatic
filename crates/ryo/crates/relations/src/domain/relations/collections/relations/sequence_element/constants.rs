//! The names of the `sequence_element` relation: the text its name and its columns carry when they leave the engine.

use super::item::SequenceElementColumn;
use crate::domain::relations::constants::{
    BOOLEAN, DATETIME, FLOAT, INTEGER, OCCURRENCE, ORDINAL, RADIX,
};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "sequence-element"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A position of a sequence of a data document."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: SequenceElementColumn) -> &'static str {
    match column {
        SequenceElementColumn::Decoded => "decoded",
        SequenceElementColumn::Kind => "kind",
        SequenceElementColumn::Occurrence => OCCURRENCE,
        SequenceElementColumn::Ordinal => ORDINAL,
        SequenceElementColumn::Owner => "owner",
        SequenceElementColumn::File => "file",
        SequenceElementColumn::Span => "span",
        SequenceElementColumn::Text => "text",
        SequenceElementColumn::Boolean => BOOLEAN,
        SequenceElementColumn::Integer => INTEGER,
        SequenceElementColumn::Float => FLOAT,
        SequenceElementColumn::Datetime => DATETIME,
        SequenceElementColumn::Radix => RADIX,
    }
}
