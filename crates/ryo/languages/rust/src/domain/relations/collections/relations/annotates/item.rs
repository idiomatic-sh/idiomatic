//! The `annotates` relation of the language: the type each named parameter, function result and single-name `let`, `const` and `static` is annotated with, as written.

use crate::domain::bindings::service::annotations;
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::service::{end, kind_bits, start, written};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `annotates` relation's rows to.
pub(crate) struct AnnotatesItem;

ryo::reads! {
    AnnotatesItem => languages::Annotates |tree, at, symbols| {
        kinds: [Kind::Node(RustNode::Fn), Kind::Node(RustNode::ClosureExpr), Kind::Node(RustNode::LetStmt), Kind::Node(RustNode::Const), Kind::Node(RustNode::Static)] as kind_bits,
        each (annotations(tree, at)) as annotated {
            row {
                occurrence: node(annotated.annotation),
                owner: node(annotated.owner),
                annotation: value(written(
                    tree,
                    symbols,
                    start(tree, annotated.annotation),
                    end(tree, annotated.annotation),
                )),
                file: file,
                span: span(annotated.annotation),
            }
        }
    }
}
