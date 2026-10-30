//! The names of the `comment` relation: the text its name and its columns carry when they leave the engine.

use super::item::CommentColumn;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "comment"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A line, block or doc comment: content without delimiters, with the full source span, and whether it is documentation."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: CommentColumn) -> &'static str {
    match column {
        CommentColumn::Id => "id",
        CommentColumn::Text => "text",
        CommentColumn::File => "file",
        CommentColumn::Span => "span",
        CommentColumn::Doc => "doc",
    }
}
