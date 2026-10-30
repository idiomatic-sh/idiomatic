//! The `syntax_error` relation of the language: every diagnostic of the reading, kept beside the tree. `CommonMark` reads every text
//! as a document, so the one error is bytes that are not UTF-8 text.

use crate::domain::model::types::Sections;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    diagnostic_id, diagnostic_message, diagnostic_span, is_root, kind_bits,
};
use crate::domain::syntax::types::{Kind, Rootness};
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_error` relation to, which reads its rows from a file's tree.
pub(crate) struct SyntaxErrorItem;

ryo::reads! {
    SyntaxErrorItem => languages::SyntaxError, model(Sections) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Root)] as kind_bits,
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
