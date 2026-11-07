//! Read only demanded numeric occurrences, preserving partiality and real source IDs.

use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    numeric_literals::{
        service::{classify, intern_value},
        types::NumericClassification,
    },
    semantic_syntax::types::Visibility,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
use super::constants::INCOMPLETE;


/// Optional numeric literal reader; no projection runs until this relation is requested.
pub(crate) struct NumericLiteralItem;

ryo::reads! {
    NumericLiteralItem => languages::NumericLiteral, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when classify(tree, expression) {
                NumericClassification::NotNumeric => {}
                NumericClassification::MissingPayload => {
                    partial INCOMPLETE => languages::Missing::Any;
                }
                NumericClassification::Known(numeric) => {
                    given kind = languages::intern_derived(symbols, tree.source, numeric.kind);
                    given value = intern_value(symbols, tree.source, &numeric);
                    row { id: node(expression), kind: value(kind), value: value(value), file: file, span: span(expression), }
                }
            }
        }
    }
}
