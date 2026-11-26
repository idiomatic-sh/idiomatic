//! TypeScript `static_call` rows.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `static-call` rows of one TypeScript file from its syntax tree.
pub(crate) struct StaticCallItem;

// The engine fills `static-call` from the walked file: every row of the file, once.
ryo::fills! {
    StaticCallItem => languages::StaticCall, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StaticCall as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::CallExpression), Kind::Node(Node::NewExpression)]) {
            let Some(callee) = names::nodes(file, at).next() else {
                continue;
            };
            rows.write(Row {
                occurrence: node_cell(file, at),
                owner: scopes::owner(file, at),
                target: names::target(file, &mut words, callee),
                file: file.file,
                span: span_at(file, callee),
            });
        }
    }
}
