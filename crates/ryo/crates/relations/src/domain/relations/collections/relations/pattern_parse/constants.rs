//! The names of the `pattern-parse` relation: the text its name and its columns carry when they leave the engine.
use super::item::PatternParseColumn;
use crate::domain::relations::constants::OUTCOME;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "pattern-parse"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "Optional parse outcomes of the regular-expression patterns source elements carry, with the flags they were read with, and rows \
     for refused carriers outside the patterns the language's constructor reads."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: PatternParseColumn) -> &'static str {
    match column {
        PatternParseColumn::Carrier => "carrier",
        PatternParseColumn::Flags => "flags",
        PatternParseColumn::Outcome => OUTCOME,
        PatternParseColumn::Error => "error",
        PatternParseColumn::File => "file",
        PatternParseColumn::Span => "span",
    }
}
