//! The `syntax_tree` relation of the language: every node and every token, the markup and the white space between the parser's
//! events included.

use crate::domain::model::types::Sections;
use crate::domain::syntax::service::{kind_symbol, token_text};
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_tree` relation to, which reads its rows from a file's tree.
pub(crate) struct SyntaxTreeItem;

ryo::reads! {
    SyntaxTreeItem => languages::SyntaxTree, model(Sections) |tree, _model, at, symbols| {
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
