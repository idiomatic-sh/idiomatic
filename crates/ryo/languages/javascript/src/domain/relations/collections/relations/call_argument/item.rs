//! JavaScript `call_argument` rows from the `ESTree` projection: each argument of a call or `new`, by its place among the arguments.
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `call-argument` rows of one JavaScript file from its syntax tree.
pub(crate) struct CallArgumentItem;
ryo::reads! {CallArgumentItem => languages::CallArgument |tree,at,symbols|{kinds:[Kind::Node(JavaScriptNode::CallExpression),Kind::Node(JavaScriptNode::NewExpression)] as kind_bits,
each (syntax::arguments_of(tree, at)) as (position, argument) {
    row {
        call: node(at),
        position: value(languages::Integer(i64::from(position.0))),
        expression: node(argument),
        passes: value(syntax::passed(tree, symbols, argument)),
        file: file,
        span: span(argument),
    }
}}}
