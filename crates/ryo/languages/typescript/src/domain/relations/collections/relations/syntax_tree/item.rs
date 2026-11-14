//! The `syntax_tree` relation of the language: every node and every token, white space and comments included.

use crate::domain::model::types::ReadIndexes;
use crate::domain::syntax::service::{kind_symbol, token_text};
use ryo::languages;

/// Reads the `syntax-tree` rows of one TypeScript file from its syntax tree.
pub(crate) struct SyntaxTreeItem;

ryo::reads! {
    SyntaxTreeItem => languages::SyntaxTree, model(ReadIndexes) |tree, _model, at, symbols| {
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
