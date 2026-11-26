//! The `syntax_error` relation of the language: every error the parser recovered from, and every token its tokenizer could not
//! read, kept beside the recovered tree.

use crate::domain::file_view::types::CssWalk;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct SyntaxErrorItem;

// The engine fills `syntax_error` from the walked file: what the parser said, in the order it said it.
ryo::fills! {
    SyntaxErrorItem => languages::SyntaxError, walk(CssWalk) |file, words, rows| {
        languages::facts::syntax_error_rows(file.syntax.table, file.syntax.file, words, rows);
    }
}
