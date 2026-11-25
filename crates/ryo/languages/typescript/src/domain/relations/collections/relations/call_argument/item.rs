//! TypeScript `call_argument` rows: each argument of a call or `new`, by its place among the arguments.
use crate::domain::arguments::service as arguments;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `call-argument` rows of one TypeScript file from its syntax tree.
pub(crate) struct CallArgumentItem;
ryo::reads! { CallArgumentItem => languages::CallArgument, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::CallExpression),Kind::Node(Node::NewExpression)] as kind_bits,
 each (arguments::arguments_of(tree, at)) as (position, argument) {
    row {
        call: node(at),
        position: value(languages::Integer(i64::from(position.0))),
        expression: node(argument),
        passes: value(arguments::passed(tree, symbols, argument)),
        file: file,
        span: span(argument),
    }
 }
} }

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
