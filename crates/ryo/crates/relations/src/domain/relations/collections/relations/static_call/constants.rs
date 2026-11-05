//! The names of the `static_call` relation: the text its name and its columns carry when they leave the engine.

use super::item::StaticCallColumn;
use crate::domain::relations::constants::{OCCURRENCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "static-call"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A call of a path written out, by the path it names."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: StaticCallColumn) -> &'static str {
    match column {
        StaticCallColumn::Occurrence => OCCURRENCE,
        StaticCallColumn::Owner => "owner",
        StaticCallColumn::Target => TARGET,
        StaticCallColumn::File => "file",
        StaticCallColumn::Span => "span",
    }
}
