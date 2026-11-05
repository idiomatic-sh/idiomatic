//! The `static_call` relation of the language: a call whose callee is a path written out, or a method called by name.

use crate::domain::nodes::types::Node;
use crate::domain::references::service::called;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `static_call` relation's rows to.
pub(crate) struct StaticCallItem;

ryo::reads! {
    StaticCallItem => languages::StaticCall |tree, at, symbols| {
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
