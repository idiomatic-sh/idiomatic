//! TypeScript inherits rows.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `inherits` rows of one TypeScript file from its syntax tree.
pub(crate) struct InheritsItem;

// The engine fills `inherits` from the walked file: every row of the file, once.
ryo::fills! {
    InheritsItem => languages::Inherits, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Inherits as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[
            Kind::Node(Node::ClassDeclaration),
            Kind::Node(Node::ClassExpression),
            Kind::Node(Node::TSInterfaceDeclaration),
        ]) {
            for base in names::bases(file, at) {
                rows.write(Row {
                    occurrence: node_cell(file, base),
                    declaration: node_cell(file, at),
                    target: names::target(file, &mut words, base),
                    file: file.file,
                    span: span_at(file, base),
                });
            }
        }
    }
}
