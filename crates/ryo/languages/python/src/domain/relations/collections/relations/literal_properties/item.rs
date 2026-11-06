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
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when classify(tree, expression) {
                LiteralClassification::NotPrimitive => {}
                LiteralClassification::MissingPayload => {
                    partial INCOMPLETE => languages::Missing::Any;
                }
                LiteralClassification::Known(properties) => {
                    given kind = languages::intern_derived(symbols, tree.source, languages::NameText(properties.kind));
                    given zero = if properties.zero { languages::Flag::Raised } else { languages::Flag::Lowered };
                    given empty = if properties.empty { languages::Flag::Raised } else { languages::Flag::Lowered };
                    given boolean = if properties.boolean { languages::Flag::Raised } else { languages::Flag::Lowered };
                    row { id: node(expression), kind: value(kind), zero: value(zero), empty: value(empty), boolean: value(boolean), file: file, span: span(expression), }
                }
            }
        }
    }
}
