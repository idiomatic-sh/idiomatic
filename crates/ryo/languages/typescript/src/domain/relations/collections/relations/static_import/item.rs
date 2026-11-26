//! TypeScript `static_import` rows from its standard syntax tree: every import declaration, every dynamic `import(..)` and every call of
//! `require`, whose binding is unknown and whose module is unknown unless a string names it.
use crate::domain::imports::service as imports;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `static-import` rows of one TypeScript file from its syntax tree.
pub(crate) struct StaticImportItem;

// The engine fills `static-import` from the walked file: every row of the file, once.
ryo::fills! {
    StaticImportItem => languages::StaticImport, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StaticImport as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[
            Kind::Node(Node::ImportDeclaration),
            Kind::Node(Node::ImportExpression),
            Kind::Node(Node::CallExpression),
        ]) {
            let Some(target) = imports::import_source(file, &mut words, at) else {
                continue;
            };
            for binding in imports::imports(file, at) {
                rows.write(Row {
                    occurrence: node_cell(file, at),
                    owner: scopes::owner(file, at),
                    binding: imports::import_name(file, &mut words, binding),
                    target,
                    file: file.file,
                    span: span_at(file, at),
                });
            }
        }
    }
}
