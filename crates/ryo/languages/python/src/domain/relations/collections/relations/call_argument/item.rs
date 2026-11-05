//! The `call_argument` relation of the language: each argument of a call, by its place among the arguments as they are written.

use crate::domain::model::types::Declarations;
use crate::domain::names::service::{arguments_of, passed};
use crate::domain::nodes::types::Node;
use crate::domain::semantic_syntax::types::Visibility;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `call-argument` rows of one Python file from its syntax tree.
pub(crate) struct CallArgumentItem;

ryo::reads! {
    CallArgumentItem => languages::CallArgument, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Call)] as kind_bits, expanded,
        require crate::domain::semantic_syntax::service::visibility(tree, model, at, symbols) == Visibility::Visible;
        each (arguments_of(tree, at)) as (position, argument) {
            row {
                call: node(at),
                position: value(languages::Integer(i64::from(position.0))),
                expression: node(argument),
                passes: value(passed(tree, argument)),
                file: file,
                span: span(argument),
            }
        }
    }
}
