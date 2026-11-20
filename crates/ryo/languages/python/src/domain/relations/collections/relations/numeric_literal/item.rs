//! Read only demanded numeric occurrences, preserving partiality and real source IDs.

use super::constants::INCOMPLETE;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::number;
use crate::domain::node::service::{fixed, node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    numeric_literals::{
        service::{classify, exactness, intern_value},
        types::NumericClassification,
    },
    semantic_syntax::types::Visibility,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;

/// Optional numeric literal reader; no projection runs until this relation is requested.
pub(crate) struct NumericLiteralItem;

ryo::reads! {
    NumericLiteralItem => languages::NumericLiteral, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when classify(tree, expression) {
                NumericClassification::NotNumeric => {}
                NumericClassification::MissingPayload => {
                    partial INCOMPLETE => languages::Missing::Any;
                }
                NumericClassification::Known(numeric) => {
                    given kind = languages::intern_derived(symbols, tree.source, numeric.kind);
                    given value = intern_value(symbols, tree.source, &numeric);
                    row { id: node(expression), kind: value(kind), value: value(value), exact_digits: value(exactness(tree, expression, &numeric)), file: file, span: span(expression), }
                }
            }
        }
    }
}

// The engine fills `numeric-literal` from the walked file: every row of the file, once.
ryo::fills! {
    NumericLiteralItem => languages::NumericLiteral, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::NumericLiteral as languages::CatalogRelation>::Row;
        let memo = &mut file.buffers.fixed;
        let value = &mut file.buffers.text;
        for Numbered { id, .. } in nodes_of_kind(&file.syntax, Kind::Node(Node::Constant)) {
            if let Some(found) = number(&file.syntax, id, value) {
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    kind: fixed(words, memo, found.kind),
                    value: word(words, languages::NameText(&value.0)),
                    exact_digits: found.exact,
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
