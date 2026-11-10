//! The names of the `analyzed_source` relation: the text its name and its columns carry when they leave the engine.

use super::item::AnalyzedSourceColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "analyzed-source"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A source file whose written paths a language examined in full."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: AnalyzedSourceColumn) -> &'static str {
    match column {
        AnalyzedSourceColumn::Path => "path",
    }
}
