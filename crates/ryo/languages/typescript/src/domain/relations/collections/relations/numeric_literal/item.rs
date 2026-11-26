//! TypeScript `numeric_literal` rows: the kind and value the parser decoded for each numeric literal, never its spelling.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at, word, written};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{NUMERIC_KIND, NUMERIC_VALUE};
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, node_table};

/// Reads the `numeric-literal` rows of one TypeScript file from the numeric properties of its literals.
pub(crate) struct NumericLiteralItem;

// The engine fills `numeric-literal` from the walked file: every row of the file, once.
ryo::fills! {
    NumericLiteralItem => languages::NumericLiteral, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::NumericLiteral as languages::CatalogRelation>::Row;
        for at in of_kinds(file, &[Kind::Node(Node::Literal)]) {
            let (Some(kind), Some(value)) = (
                node_table::property_of(file.table, at, NUMERIC_KIND),
                node_table::property_of(file.table, at, NUMERIC_VALUE),
            ) else {
                continue;
            };
            rows.write(Row {
                id: node_cell(file, at),
                kind: word(words, kind),
                value: word(words, value),
                exact_digits: crate::domain::syntax::service::exactness_of(Some(kind), Some(value), written(file, at).0),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}
