//! The `call_argument` relation of the language: each argument of a call whose callee is a path written out, or of a method called by
//! name, by its place among the arguments.

use crate::domain::nodes::types::Node;
use crate::domain::references::service::{arguments_of, called, passed};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::macros::fills_rows;
use ryo::languages;

/// The reader the `fills_rows!` below binds the `call_argument` relation's rows to.
pub(crate) struct CallArgumentItem;

// The engine fills the relation from the walked file, by the same reader.
fills_rows! {
    CallArgumentItem => languages::CallArgument, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::CallExpr), Kind::Node(Node::MethodCallExpr)] as kind_bits,
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
