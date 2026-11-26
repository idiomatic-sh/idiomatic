//! Native HTML tokenizer and tree-construction diagnostics, retained beside the recovered DOM.

use crate::domain::file_view::types::HtmlWalk;
use ryo::languages;

/// The item the engine's fill of the `syntax_error` relation is registered on.
pub(crate) struct SyntaxErrorItem;

// The engine fills `syntax_error` from the walked file: what the parser said, in the order it said it.
ryo::fills! {
    SyntaxErrorItem => languages::SyntaxError, walk(HtmlWalk) |file, words, rows| {
        languages::facts::syntax_error_rows(file.table, file.file, words, rows);
    }
}

