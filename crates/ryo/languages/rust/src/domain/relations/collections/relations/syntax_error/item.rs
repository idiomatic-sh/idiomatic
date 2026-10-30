//! The `syntax_error` relation of the language: every diagnostic of the parser, kept beside the recovered tree.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    diagnostic_id, diagnostic_message, diagnostic_span, is_root, kind_bits,
};
use crate::domain::syntax::types::{Kind, Rootness};
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `syntax_error` relation's rows to.
pub(crate) struct SyntaxErrorItem;

ryo::reads! {
    SyntaxErrorItem => languages::SyntaxError |tree, at, symbols| {
        kinds: [Kind::Node(Node::SourceFile)] as kind_bits,
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
