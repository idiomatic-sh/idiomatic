//! The `syntax_error` relation of the language: every diagnostic of the parser, kept beside the recovered tree.

use crate::domain::file_view::types::TomlWalk;
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_error` relation to, which reads its rows from a file's tree.
pub(crate) struct SyntaxErrorItem;


// The engine fills `syntax_error` from the walked file: what the parser said, in the order it said it.
ryo::fills! {
    SyntaxErrorItem => languages::SyntaxError, walk(TomlWalk) |file, words, rows| {
        languages::facts::syntax_error_rows(file.syntax.table, file.syntax.file, words, rows);
    }
}
