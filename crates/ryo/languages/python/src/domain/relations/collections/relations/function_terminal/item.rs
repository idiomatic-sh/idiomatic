//! Read one terminal outcome for every visible function definition.
use super::constants::INCOMPLETE;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::node::service::{fixed, node_cell, nodes_of_kind, span_at};
use crate::domain::node::types::Numbered;
use crate::domain::terminal_facts::service::outcome as ending;
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    syntax::{service::kind_bits, types::Kind},
    terminal_flow::{
        service::{outcome, prepare},
        types::{IndexState, TerminalIndex},
    },
};
use ryo::languages;

/// Reads one terminal outcome for each visible function.
pub(crate) struct FunctionTerminalItem;

/// Return whether the terminal index covers the current source.
fn state(index: &TerminalIndex) -> IndexState {
    index.state
}

ryo::reads! {
    FunctionTerminalItem => languages::FunctionTerminal, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(tree, &model.terminal_flow);
        when state(&index) {
            IndexState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            IndexState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            IndexState::Complete => {}
        }
        each (crate::domain::semantic_syntax::service::positions(tree)) as function {
            require matches!(crate::domain::syntax::service::kind(tree,function), Kind::Node(Node::FunctionDef|Node::AsyncFunctionDef));
            require crate::domain::semantic_syntax::service::visibility(tree, model, function, symbols) == Visibility::Visible;
            given outcome = outcome(&index,function)?;
            given outcome = languages::intern_derived(symbols, tree.source, outcome);
            row { id: node(function), outcome: value(outcome), file: file, span: span(function), }
        }
    }
}

// The engine fills `function-terminal` from the walked file: every row of the file, once.
ryo::fills! {
    FunctionTerminalItem => languages::FunctionTerminal, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FunctionTerminal as languages::CatalogRelation>::Row;
        if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
        let memo = &mut file.buffers.fixed;
        for kind in [Node::FunctionDef, Node::AsyncFunctionDef] {
            for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                let ruff_python_ast::AnyNodeRef::StmtFunctionDef(function) = node else {
                    continue;
                };
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    outcome: fixed(words, memo, ending(function)),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
