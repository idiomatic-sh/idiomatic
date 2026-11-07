//! Emit optional primitive-literal properties from parser-owned payloads.

use crate::domain::{
    literal_properties::{service::classify, types::LiteralClassification},
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
use super::constants::INCOMPLETE;


/// The optional Python primitive-literal reader.
pub(crate) struct LiteralPropertiesItem;

ryo::reads! {
    LiteralPropertiesItem => languages::LiteralProperties, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when classify(tree, expression) {
                LiteralClassification::NotPrimitive => {}
                LiteralClassification::MissingPayload => {
                    partial INCOMPLETE => languages::Missing::Any;
                }
                LiteralClassification::Known(properties) => {
                    given kind = languages::intern_derived(symbols, tree.source, properties.kind);
                    given zero = properties.zero;
                    given empty = properties.empty;
                    given boolean = properties.boolean;
                    row { id: node(expression), kind: value(kind), zero: value(zero), empty: value(empty), boolean: value(boolean), file: file, span: span(expression), }
                }
            }
        }
    }
}
