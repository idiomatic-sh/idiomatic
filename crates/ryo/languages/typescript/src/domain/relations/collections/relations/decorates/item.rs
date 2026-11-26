//! TypeScript decorates rows.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::service::parent;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `decorates` rows of one TypeScript file from its syntax tree.
pub(crate) struct DecoratesItem;

// The engine fills `decorates` from the walked file: every row of the file, once.
ryo::fills! {
    DecoratesItem => languages::Decorates, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Decorates as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::Decorator)]) {
            let declaration = parent(file, at);
            rows.write(Row {
                occurrence: node_cell(file, at),
                declaration: node_cell(file, declaration),
                ordinal: languages::Integer(
                    i64::try_from(
                        names::nodes(file, declaration)
                            .take_while(|child| *child != at)
                            .filter(|child| kind(file, *child) == Kind::Node(Node::Decorator))
                            .count(),
                    )
                    .unwrap_or(i64::MAX),
                ),
                target: names::target(file, &mut words, at),
                arguments: crate::domain::decorations::service::arguments(file, &mut words, at),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}
