//! TypeScript annotates rows.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::parent;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `annotates` rows of one TypeScript file from its syntax tree.
pub(crate) struct AnnotatesItem;
ryo::reads! { AnnotatesItem => languages::Annotates, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::TSTypeAnnotation)] as kind_bits,
 given annotation = names::nodes(tree,at).next()?;
 row { occurrence: node(at),owner: node(parent(tree, at)),annotation: value(names::written(tree,symbols,annotation)),file: file,span: span(annotation) }

} }

// The engine fills `annotates` from the walked file: every row of the file, once.
ryo::fills! {
    AnnotatesItem => languages::Annotates, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Annotates as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::TSTypeAnnotation)]) {
            let Some(annotation) = names::nodes(file, at).next() else {
                continue;
            };
            rows.write(Row {
                occurrence: node_cell(file, at),
                owner: node_cell(file, parent(file, at)),
                annotation: names::written(file, &mut words, annotation),
                file: file.file,
                span: span_at(file, annotation),
            });
        }
    }
}
