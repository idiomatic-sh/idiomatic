//! The names of the `syntax_error` relation: the text its name and its columns carry when they leave the engine.

use super::item::SyntaxErrorColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "syntax-error"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A diagnostic of the parser, kept beside the recovered tree."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: SyntaxErrorColumn) -> &'static str {
    match column {
        SyntaxErrorColumn::Id => "id",
        SyntaxErrorColumn::Message => "message",
        SyntaxErrorColumn::File => "file",
        SyntaxErrorColumn::Span => "span",
    }
}
