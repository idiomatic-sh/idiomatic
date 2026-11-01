//! Read supported constant-expression equivalence classes.

use crate::domain::{
    constant_keys::{
        constants::INCOMPLETE,
        service::{key_of, prepare},
        types::{ConstantKeyIndex, KeyState},
    },
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    syntax::service::kind_bits,
    syntax::types::Kind,
};
use ryo::languages;

pub(crate) struct ConstantKeyItem;
fn state(index: &ConstantKeyIndex) -> KeyState {
    index.state
}

ryo::reads! {
    ConstantKeyItem => languages::ConstantKey, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare(tree, model, symbols);
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            given key = key_of(&index, expression)?;
            row { id: node(expression), key: node(key), file: file, span: span(expression), }
        }
        when state(&index) {
            KeyState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            KeyState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            KeyState::Complete => {}
        }
    }
}
