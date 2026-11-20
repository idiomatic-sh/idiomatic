//! The `function` relation of the language: every `def`, async or not, at any depth, methods included; a lambda is no function.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::types::Declarations;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind_bits, name_of};
use crate::domain::syntax::types::Kind;
use ruff_python_ast::AnyNodeRef;
use ryo::languages;

/// Reads the `function` rows of one Python file from its syntax tree.
pub(crate) struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function, model(Declarations) |tree, model, at, _symbols| {
        kinds: [Kind::Node(Node::FunctionDef), Kind::Node(Node::AsyncFunctionDef)] as kind_bits,
        given identifier = child(tree, at, Kind::Extra(Extra::Identifier))?;
        given name = name_of(tree, identifier)?;
        row { id: node(at), name: value(name), file: file, span: span(at) }
    }
}

// The engine fills `function` from the walked file: every row of the file, once.
ryo::fills! {
    FunctionItem => languages::Function, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Function as languages::CatalogRelation>::Row;
        for kind in [Node::FunctionDef, Node::AsyncFunctionDef] {
            for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                let AnyNodeRef::StmtFunctionDef(function) = node else {
                    continue;
                };
                if function.name.id.is_empty() {
                    continue;
                }
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    name: word(words, languages::NameText(function.name.id.as_str())),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
