//! The `call_argument` relation of the language: each argument of a call, by its place among the arguments as they are written.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::types::Declarations;
use crate::domain::name_facts::service::passed as handed_on;
use crate::domain::names::service::{arguments_of, passed};
use crate::domain::node::service::{element_id, node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;
use crate::domain::semantic_syntax::types::Visibility;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ruff_python_ast::{AnyNodeRef, ArgOrKeyword};
use ryo::languages;

/// Reads the `call-argument` rows of one Python file from its syntax tree.
pub(crate) struct CallArgumentItem;

ryo::reads! {
    CallArgumentItem => languages::CallArgument, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Call)] as kind_bits, expanded,
        require crate::domain::semantic_syntax::service::visibility(tree, model, at, symbols) == Visibility::Visible;
        each (arguments_of(tree, at)) as (position, argument) {
            row {
                call: node(at),
                position: value(languages::Integer(i64::from(position.0))),
                expression: node(argument),
                passes: value(passed(tree, argument)),
                file: file,
                span: span(argument),
            }
        }
    }
}

// The engine fills `call-argument` from the walked file: every row of the file, once.
ryo::fills! {
    CallArgumentItem => languages::CallArgument, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::CallArgument as languages::CatalogRelation>::Row;
        for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(Node::Call)) {
            let AnyNodeRef::ExprCall(call) = node else {
                continue;
            };
            for (position, argument) in call.arguments.iter_source_order().enumerate() {
                let expression = match argument {
                    ArgOrKeyword::Arg(expression) => expression,
                    ArgOrKeyword::Keyword(keyword) => &keyword.value,
                };
                let Some(at) = element_id(&file.syntax, expression) else {
                    continue;
                };
                rows.write(Row {
                    call: node_cell(&file.syntax, id),
                    position: languages::Integer(i64::try_from(position).unwrap_or(i64::MAX)),
                    expression: node_cell(&file.syntax, at),
                    passes: handed_on(expression)
                        .map_or(languages::NO_SYMBOL, |name| word(words, name)),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, at),
                });
            }
        }
    }
}
