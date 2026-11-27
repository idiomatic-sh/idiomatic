//! The names of the `semantic_syntax` relation: the text its name and its columns carry when they leave the engine.

use super::item::SemanticSyntaxColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "semantic-syntax"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "Native syntax and parsed embedded expressions; token text is logical and spans retain physical source locations."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: SemanticSyntaxColumn) -> &'static str {
    match column {
        SemanticSyntaxColumn::Id => "id",
        SemanticSyntaxColumn::Kind => "kind",
        SemanticSyntaxColumn::Text => "text",
        SemanticSyntaxColumn::File => "file",
        SemanticSyntaxColumn::Span => "span",
    }
}
