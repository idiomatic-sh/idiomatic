//! The names of the `syntax_tree` relation: the text its name and its columns carry when they leave the engine.

use super::item::SyntaxTreeColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "syntax-tree"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A node or a token of the syntax tree, whitespace and comments included."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: SyntaxTreeColumn) -> &'static str {
    match column {
        SyntaxTreeColumn::Id => "id",
        SyntaxTreeColumn::Kind => "kind",
        SyntaxTreeColumn::Text => "text",
        SyntaxTreeColumn::File => "file",
        SyntaxTreeColumn::Span => "span",
    }
}
