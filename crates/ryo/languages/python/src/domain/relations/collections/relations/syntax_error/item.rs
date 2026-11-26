//! The `syntax_error` relation of the language: every diagnostic of the parser, kept beside the recovered tree.

use crate::domain::file_view::types::PythonWalk;




use ryo::languages;

/// Reads the `syntax-error` rows of one Python file from its syntax tree.
pub(crate) struct SyntaxErrorItem;

// The engine fills `syntax-error` from the walked file: every row of the file, once.
ryo::fills! {
    SyntaxErrorItem => languages::SyntaxError, walk(PythonWalk) |file, words, rows| {
        // One row for each thing the parser and the language said of the file, in the order the walk recorded them.
        languages::facts::syntax_error_rows(file.syntax.table, file.syntax.file, words, rows);
    }
}
