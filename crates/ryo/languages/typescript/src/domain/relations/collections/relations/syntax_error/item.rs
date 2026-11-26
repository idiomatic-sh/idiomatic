//! The parser's diagnostics, including malformed UTF-8 and recovered syntax.

use crate::domain::file_view::types::TypeScriptWalk;
use ryo::languages;

/// Reads the `syntax-error` rows of one TypeScript file from its syntax tree.
pub(crate) struct SyntaxErrorItem;


// The engine fills `syntax-error` from the walked file: every diagnostic the parser reported, once.
ryo::fills! {
    SyntaxErrorItem => languages::SyntaxError, walk(TypeScriptWalk) |file, words, rows| {
        languages::facts::syntax_error_rows(file.table, file.file, words, rows);
    }
}
