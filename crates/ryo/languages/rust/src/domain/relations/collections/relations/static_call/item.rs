//! The `static_call` relation of the language: a call whose callee is a path written out, or a method called by name.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::references::service::called;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::macros::fills_rows;
use ryo::languages;

/// The reader the `fills_rows!` below binds the `static_call` relation's rows to.
pub(crate) struct StaticCallItem;

// The engine fills the relation from the walked file, by the same reader.
fills_rows! {
    StaticCallItem => languages::StaticCall, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::CallExpr), Kind::Node(Node::MethodCallExpr)] as kind_bits,
        given call = called(tree, symbols, at)?;
        row {
            occurrence: node(call.callee),
            owner: value(owner(tree, at)),
            target: value(call.target),
            file: file,
            span: span(call.callee),
        }
    }
}
