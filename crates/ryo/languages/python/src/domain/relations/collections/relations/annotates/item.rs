//! The `annotates` relation of the language: the type of each annotated parameter, the result of each annotated function and the
//! type of each annotated assignment target, as written; a quoted annotation is its text with the quotes.

use crate::domain::model::types::Declarations;
use crate::domain::names::service::annotation_of;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, spanned};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `annotates` rows of one Python file from its syntax tree.
pub(crate) struct AnnotatesItem;

ryo::reads! {
    AnnotatesItem => languages::Annotates, model(Declarations) |tree, model, at, symbols| {
        kinds: [
            Kind::Node(Node::Arg),
            Kind::Node(Node::FunctionDef),
            Kind::Node(Node::AsyncFunctionDef),
            Kind::Node(Node::AnnAssign),
        ] as kind_bits,
        given annotated = annotation_of(tree, at)?;
        row {
            occurrence: node(annotated.annotation),
            owner: node(annotated.target),
            annotation: value(spanned(tree, symbols, annotated.annotation)),
            file: file,
            span: span(annotated.annotation),
        }
    }
}
