//! Native HTML tokenizer and tree-construction diagnostics, retained beside the recovered DOM.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    diagnostic_id, diagnostic_message, diagnostic_span, is_root, kind_bits,
};
use crate::domain::syntax::types::{Kind, Rootness};
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_error` relation to, which reads its rows from a file's tree.
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

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
