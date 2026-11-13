//! TypeScript `call_argument` rows: each argument of a call or `new`, by its place among the arguments.
use crate::domain::arguments::service as arguments;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `call-argument` rows of one TypeScript file from its syntax tree.
pub(crate) struct CallArgumentItem;
ryo::reads! { CallArgumentItem => languages::CallArgument, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::CallExpression),Kind::Node(Node::NewExpression)] as kind_bits,
 each (arguments::arguments_of(tree, at)) as (position, argument) {
    row {
        call: node(at),
        position: value(languages::Integer(i64::from(position.0))),
        expression: node(argument),
        passes: value(arguments::passed(tree, symbols, argument)),
        file: file,
        span: span(argument),
    }
 }
} }
