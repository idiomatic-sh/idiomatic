//! The `syntax_error` relation of the language: every diagnostic of the parser, kept beside the recovered tree.

use crate::domain::model::types::Model;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    diagnostic_id, diagnostic_message, diagnostic_span, is_root, kind_bits,
};
use crate::domain::syntax::types::{Kind, Rootness};
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_error` relation to, which reads its rows from a file's tree.
pub(crate) struct SyntaxErrorItem;

ryo::reads! {
    SyntaxErrorItem => languages::SyntaxError, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Toml)] as kind_bits,
        require (is_root(tree, at) == Rootness::Root);
        each (languages::diagnostics(tree.elements)) as (number, diagnostic) {
            row {
                id: value(diagnostic_id(tree, number)),
                message: value(diagnostic_message(tree, symbols, &diagnostic)),
                file: file,
                span: value(diagnostic_span(&diagnostic)),
            }
        }
    }
}
