//! The names of the `pattern-child` relation: the text its name and its columns carry when they leave the engine.
use super::item::PatternChildColumn;
use crate::domain::relations::constants::{ORDINAL, PARENT};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "pattern-child"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "Optional edges of the parsed regular-expression patterns source elements carry, ordered under their parent, with the child's role."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: PatternChildColumn) -> &'static str {
    match column {
        PatternChildColumn::Parent => PARENT,
        PatternChildColumn::Child => "child",
        PatternChildColumn::Ordinal => ORDINAL,
        PatternChildColumn::Field => "field",
        PatternChildColumn::File => "file",
    }
}
