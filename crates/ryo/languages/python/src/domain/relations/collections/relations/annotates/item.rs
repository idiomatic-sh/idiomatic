//! The `annotates` relation of the language: the type of each annotated parameter, the result of each annotated function and the
//! type of each annotated assignment target, as written; a quoted annotation is its text with the quotes.

use crate::domain::model::types::Declarations;
use crate::domain::names::service::annotation_of;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, spanned};
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::node::service::{expressions_of, node_cell, nodes_of_kind, span_at, text_between, word};
use crate::domain::node::types::Numbered;
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

// The engine fills `annotates` from the walked file: every annotated parameter, function and assignment, once.
ryo::fills! {
    AnnotatesItem => languages::Annotates, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Annotates as languages::CatalogRelation>::Row;
        for kind in [Node::Arg, Node::FunctionDef, Node::AsyncFunctionDef, Node::AnnAssign] {
            for Numbered { id, .. } in nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                let mut expressions = expressions_of(&file.syntax, id);
                let (target, annotation) = if kind == Node::AnnAssign {
                    let (Some(target), Some(annotation)) = (expressions.next(), expressions.next()) else {
                        continue;
                    };
                    (target, annotation)
                } else {
                    let Some(annotation) = expressions.next() else {
                        continue;
                    };
                    (id, annotation)
                };
                let span = span_at(&file.syntax, annotation);
                rows.write(Row {
                    occurrence: node_cell(&file.syntax, annotation),
                    owner: node_cell(&file.syntax, target),
                    annotation: word(words, text_between(&file.syntax, span.start, span.end)),
                    file: file.syntax.file,
                    span,
                });
            }
        }
    }
}
