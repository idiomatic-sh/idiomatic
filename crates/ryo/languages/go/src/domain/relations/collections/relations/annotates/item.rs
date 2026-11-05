//! The `annotates` relation of the language: the type of each named parameter, result, receiver and field, and of each name a `var` or
//! `const` gives a type, as written; for what has no name (a result of a type alone), the type of the field.

use crate::domain::definitions::service::annotations;
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::service::{kind_bits, spanned};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `annotates` relation to, which reads its rows from a file's tree.
pub(crate) struct AnnotatesItem;

ryo::reads! {
    AnnotatesItem => languages::Annotates |tree, at, symbols| {
        kinds: [Kind::Node(GoNode::Field), Kind::Node(GoNode::ValueSpec)] as kind_bits,
        each (annotations(tree, at)) as annotated {
            row {
                occurrence: node(annotated.annotation),
                owner: node(annotated.target),
                annotation: value(spanned(tree, symbols, annotated.annotation)),
                file: file,
                span: span(annotated.annotation),
            }
        }
    }
}
