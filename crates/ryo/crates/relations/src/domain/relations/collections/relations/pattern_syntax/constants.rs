//! The names of the `pattern-syntax` relation: the text its name and its columns carry when they leave the engine.
use super::item::PatternSyntaxColumn;
use crate::domain::relations::constants::KIND;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "pattern-syntax"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "Optional elements of the parsed regular-expression patterns source elements carry, with the value a character or reference stands for."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: PatternSyntaxColumn) -> &'static str {
    match column {
        PatternSyntaxColumn::Id => "id",
        PatternSyntaxColumn::Carrier => "carrier",
        PatternSyntaxColumn::Kind => KIND,
        PatternSyntaxColumn::Text => "text",
        PatternSyntaxColumn::Value => "value",
        PatternSyntaxColumn::File => "file",
        PatternSyntaxColumn::Span => "span",
    }
}
