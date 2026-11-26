//! TypeScript `call_argument` rows: each argument of a call or `new`, by its place among the arguments.
use crate::domain::arguments::service as arguments;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `call-argument` rows of one TypeScript file from its syntax tree.
pub(crate) struct CallArgumentItem;

// The engine fills `call-argument` from the walked file: every row of the file, once.
ryo::fills! {
    CallArgumentItem => languages::CallArgument, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::CallArgument as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::CallExpression), Kind::Node(Node::NewExpression)]) {
            for (position, argument) in arguments::arguments_of(file, at).enumerate() {
                rows.write(Row {
                    call: node_cell(file, at),
                    position: languages::Integer(i64::try_from(position).unwrap_or(i64::MAX)),
                    expression: node_cell(file, argument),
                    passes: arguments::passed(file, &mut words, argument),
                    file: file.file,
                    span: span_at(file, argument),
                });
            }
        }
    }
}
