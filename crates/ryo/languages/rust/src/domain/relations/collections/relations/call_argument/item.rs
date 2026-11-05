//! The `call_argument` relation of the language: each argument of a call whose callee is a path written out, or of a method called by
//! name, by its place among the arguments.

use crate::domain::nodes::types::RustNode;
use crate::domain::references::service::{arguments_of, called, passed};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `call_argument` relation's rows to.
pub(crate) struct CallArgumentItem;

ryo::reads! {
    CallArgumentItem => languages::CallArgument |tree, at, symbols| {
        kinds: [Kind::Node(RustNode::CallExpr), Kind::Node(RustNode::MethodCallExpr)] as kind_bits,
        given call = called(tree, symbols, at)?;
        each (arguments_of(tree, at)) as (position, argument) {
            row {
                call: node(call.callee),
                position: value(languages::Integer(i64::from(position.0))),
                expression: node(argument),
                passes: value(passed(tree, argument)),
                file: file,
                span: span(argument),
            }
        }
    }
}
