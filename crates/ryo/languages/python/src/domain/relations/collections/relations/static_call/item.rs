//! The `static_call` relation of the language: every call, by the dotted name it calls; a callee that is no dotted name has the
//! unknown value of a dynamic callee.



use crate::domain::names::types::Unresolved;

use crate::domain::nodes::types::Node;



use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::view::service::built;
use crate::domain::model::view::types::ModelPart;
use crate::domain::name_facts::service::dotted;
use crate::domain::names::types::Dottedness;
use crate::domain::names::service::unknown;
use crate::domain::node::service::{expressions_of, node_cell, nodes_of_kind, scope_id, span_at, word};
use crate::domain::node::types::Numbered;
use ruff_python_ast::AnyNodeRef;
use ryo::languages;

/// Reads the `static-call` rows of one Python file from its syntax tree.
pub(crate) struct StaticCallItem;

// The engine fills `static-call` from the walked file: every call, by the dotted name it calls.
ryo::fills! {
    StaticCallItem => languages::StaticCall, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StaticCall as languages::CatalogRelation>::Row;
        built(file.model, &mut file.built, ModelPart::Lexical, file.syntax.tree);
        let scope_of = &file.model.lexical.scope_of;
        let names = &mut file.buffers.text;
        for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(Node::Call)) {
            let Some(callee) = expressions_of(&file.syntax, id).next() else {
                continue;
            };
            let AnyNodeRef::ExprCall(call) = node else {
                continue;
            };
            names.0.clear();
            let target = if dotted(&call.func, names) == Dottedness::Dotted {
                word(words, languages::NameText(&names.0))
            } else {
                unknown(Unresolved::DynamicCallee)
            };
            rows.write(Row {
                occurrence: node_cell(&file.syntax, id),
                owner: node_cell(&file.syntax, scope_id(&file.syntax, scope_of, id)),
                target,
                file: file.syntax.file,
                span: span_at(&file.syntax, callee),
            });
        }
    }
}
