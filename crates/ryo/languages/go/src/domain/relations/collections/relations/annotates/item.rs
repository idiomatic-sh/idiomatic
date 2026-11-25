//! The `annotates` relation of the language: the type of each named parameter, result, receiver and field, and of each name a `var` or
//! `const` gives a type, as written; for what has no name (a result of a type alone), the type of the field.

use crate::domain::definitions::service::annotations;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, spanned};
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `annotates` relation to, which reads its rows from a file's tree.
pub(crate) struct AnnotatesItem;

ryo::reads! {
    AnnotatesItem => languages::Annotates |tree, at, symbols| {
        kinds: [Kind::Node(Node::Field), Kind::Node(Node::ValueSpec)] as kind_bits,
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

// The engine fills `annotates` from the walked file: the type that annotates each name a field or a declaration declares.
ryo::fills! {
    AnnotatesItem => languages::Annotates, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Annotates as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for wanted in [Kind::Node(Node::Field), Kind::Node(Node::ValueSpec)] {
            for &at in node_table::ids_of_kind(tree.table, kind_bits(wanted)) {
                for annotated in crate::domain::walked::definitions::annotations(&tree, at) {
                    rows.write(Row {
                        occurrence: walked_syntax::node_cell(&tree, annotated.annotation),
                        owner: walked_syntax::node_cell(&tree, annotated.target),
                        annotation: walked_syntax::spanned(&tree, &words, annotated.annotation),
                        file: tree.file,
                        span: walked_syntax::span_of(&tree, annotated.annotation),
                    });
                }
            }
        }
    }
}
