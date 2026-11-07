//! The `syntax_tree` relation of the language: every node and every token, whitespace and comments included.

use crate::domain::syntax::service::{kind_symbol, token_text};
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `syntax_tree` relation's rows to.
pub(crate) struct SyntaxTreeItem;

ryo::reads! {
    SyntaxTreeItem => languages::SyntaxTree |tree, at, symbols| {
        kinds: every,
        row {
            id: node(at),
            kind: value(kind_symbol(tree, symbols, at)),
            text: value(token_text(tree, at)),
            file: file,
            span: span(at),
        }
    }
}
