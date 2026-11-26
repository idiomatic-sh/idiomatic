//! The `syntax_error` relation of the language: every diagnostic of the reading, kept beside the tree. `CommonMark` reads every text
//! as a document, so the one error is bytes that are not UTF-8 text.

use crate::domain::file_view::types::MarkdownWalk;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct SyntaxErrorItem;

// The engine fills `syntax_error` from the walked file: what the parser said, in the order it said it.
ryo::fills! {
    SyntaxErrorItem => languages::SyntaxError, walk(MarkdownWalk) |file, words, rows| {
        languages::facts::syntax_error_rows(file.syntax.table, file.syntax.file, words, rows);
    }
}
