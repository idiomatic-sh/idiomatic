//! typescript `directive` rows: the directives of a prologue, by the word between their quotes.
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::prologues::service::{argument, directive};
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `directive` rows of one TypeScript file from its syntax tree.
pub(crate) struct DirectiveItem;
ryo::reads! {
    DirectiveItem => languages::Directive, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::ExpressionStatement)] as kind_bits,
        given word = directive(tree, symbols, at)?;
        row { file: file, span: span(at), kind: value(word), text: value(argument(tree, symbols)) }
    }
}
