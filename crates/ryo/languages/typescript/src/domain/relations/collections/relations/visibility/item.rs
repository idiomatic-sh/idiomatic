//! TypeScript visibility rows.
use crate::domain::file_view::service::{node_cell, of_kinds};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::visibilities::service as visibilities;
use ryo::languages;

/// Reads the `visibility` rows of one TypeScript file from its syntax tree.
pub(crate) struct VisibilityItem;

// The engine fills `visibility` from the walked file: every row of the file, once.
ryo::fills! {
    VisibilityItem => languages::Visibility, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Visibility as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[
            Kind::Node(Node::ExportNamedDeclaration),
            Kind::Node(Node::ExportDefaultDeclaration),
            Kind::Node(Node::MethodDefinition),
            Kind::Node(Node::PropertyDefinition),
            Kind::Node(Node::AccessorProperty),
        ]) {
            for visible in visibilities::visible(file, at) {
                rows.write(Row {
                    occurrence: node_cell(file, visible.occurrence),
                    declaration: node_cell(file, visible.declaration),
                    text: visibilities::written(file, &mut words, &visible),
                    file: file.file,
                    span: visibilities::span(&visible),
                });
            }
        }
    }
}
