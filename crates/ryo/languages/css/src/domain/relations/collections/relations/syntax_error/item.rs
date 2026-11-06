//! The `syntax_error` relation of the language: every error the parser recovered from, and every token its tokenizer could not
//! read, kept beside the recovered tree.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{INCOMPLETE_IMPACT, PARSE_PHASE, UNSPECIFIED_CODE};
use crate::domain::syntax::service::{
    diagnostic_id, diagnostic_message, diagnostic_span, is_root, kind_bits,
};
use crate::domain::syntax::types::{Kind, Rootness};
use ryo::languages;

/// The reader the `parse` contract hands the `syntax_error` relation to, which reads its rows from a file's tree.
pub(crate) struct SyntaxErrorItem;

ryo::reads! {
    SyntaxErrorItem => languages::SyntaxError |tree, at, symbols| {
        kinds: [Kind::Node(Node::Stylesheet)] as kind_bits,
        require (is_root(tree, at) == Rootness::Root);
        each (languages::diagnostics(tree.elements)) as (number, diagnostic) {
            given phase = languages::intern_derived(symbols, tree.source, languages::NameText(PARSE_PHASE));
            given impact = languages::intern_derived(symbols, tree.source, languages::NameText(INCOMPLETE_IMPACT));
            given code = languages::intern_derived(symbols, tree.source, languages::NameText(UNSPECIFIED_CODE));
            row {
                id: value(diagnostic_id(tree, number)),
                message: value(diagnostic_message(tree, symbols, &diagnostic)),
                phase: value(phase), impact: value(impact), code: value(code), file: file,
                span: value(diagnostic_span(&diagnostic)),
            }
        }
    }
}
