//! The `function` relation of the language: every `def`, async or not, at any depth, methods included; a lambda is no function.


use crate::domain::file_view::types::PythonWalk;

use crate::domain::node::service::{keyed_node, node_at, node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::nodes::types::Node;

use crate::domain::syntax::types::Kind;
use ruff_python_ast::AnyNodeRef;
use ryo::languages;

/// Reads the `function` rows of one Python file from its syntax tree.
pub(crate) struct FunctionItem;

/// The row of the function definition `node` at `id`; none for any other node and for a definition the parser gave no name.
fn function_row(
    file: &SyntaxFile<'_>,
    words: &mut languages::FileWords,
    Numbered { id, node }: Numbered<'_>,
) -> Option<languages::FunctionRow> {
    let AnyNodeRef::StmtFunctionDef(function) = node else {
        return None;
    };
    (!function.name.id.is_empty()).then(|| languages::FunctionRow {
        id: node_cell(file, id),
        name: word(words, languages::NameText(function.name.id.as_str())),
        file: file.file,
        span: span_at(file, id),
    })
}

// The engine fills `function` from the walked file: every row of the file, or the row of one definition, read from its node.
ryo::fills! {
    FunctionItem => languages::Function, walk(PythonWalk) |file, words, rows| {
        for kind in [Node::FunctionDef, Node::AsyncFunctionDef] {
            for numbered in nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                if let Some(row) = function_row(&file.syntax, words, numbered) {
                    rows.write(row);
                }
            }
        }
    }
    key(languages::Column::Function(languages::FunctionColumn::Id)) |file, words, key, rows| {
        let found = keyed_node(&file.syntax, key)
            .and_then(|id| node_at(&file.syntax, id).map(|node| Numbered { id, node }))
            .and_then(|numbered| function_row(&file.syntax, words, numbered));
        if let Some(row) = found {
            rows.write(row);
        }
    }
}
