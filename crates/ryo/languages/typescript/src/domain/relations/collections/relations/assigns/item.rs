//! TypeScript assigns rows.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `assigns` rows of one TypeScript file from its syntax tree.
pub(crate) struct AssignsItem;

// The engine fills `assigns` from the walked file: every row of the file, once.
ryo::fills! {
    AssignsItem => languages::Assigns, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Assigns as languages::CatalogRelation>::Row;
        let _ = words;
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::VariableDeclarator), Kind::Node(Node::AssignmentExpression)]) {
            let Some(assigned) = names::assignment(file, at) else {
                continue;
            };
            rows.write(Row {
                occurrence: node_cell(file, at),
                binding: node_cell(file, assigned.binding),
                value: node_cell(file, assigned.value),
                owner: scopes::owner(file, at),
                file: file.file,
                span: span_at(file, assigned.value),
            });
        }
    }
}
