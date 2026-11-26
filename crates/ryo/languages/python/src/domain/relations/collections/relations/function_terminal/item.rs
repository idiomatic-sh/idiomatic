//! Read one terminal outcome for every visible function definition.
use super::constants::INCOMPLETE;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at};
use crate::domain::node::types::Numbered;
use crate::domain::terminal_facts::service::outcome as ending;
use crate::domain::{nodes::types::Node, syntax::{types::Kind}};
use ryo::languages;

/// Reads one terminal outcome for each visible function.
pub(crate) struct FunctionTerminalItem;

// The engine fills `function-terminal` from the walked file: every row of the file, once.
ryo::fills! {
    FunctionTerminalItem => languages::FunctionTerminal, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FunctionTerminal as languages::CatalogRelation>::Row;
        if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
        let memo = &mut file.model.words.get_mut().fixed;
        for kind in [Node::FunctionDef, Node::AsyncFunctionDef] {
            for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                let ruff_python_ast::AnyNodeRef::StmtFunctionDef(function) = node else {
                    continue;
                };
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    outcome: fixed_word(words, memo, ending(function)),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
