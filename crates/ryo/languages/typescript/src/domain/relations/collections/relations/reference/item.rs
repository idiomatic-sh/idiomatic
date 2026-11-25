//! TypeScript reference rows.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `reference` rows of one TypeScript file from its syntax tree.
pub(crate) struct ReferenceItem;
ryo::reads! { ReferenceItem => languages::Reference, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::Identifier),Kind::Node(Node::MemberExpression),Kind::Node(Node::TSQualifiedName)] as kind_bits,
 require names::referenced(tree,at) == crate::domain::names::types::Reference::Use;
 row { occurrence: node(at),owner: value(scopes::owner(tree,at)),target: value(names::target(tree,symbols,at)),file: file,span: span(at) }

} }

// The engine fills `reference` from the walked file: every row of the file, once.
ryo::fills! {
    ReferenceItem => languages::Reference, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Reference as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::Identifier), Kind::Node(Node::MemberExpression), Kind::Node(Node::TSQualifiedName)]) {
            if names::referenced(file, at) != crate::domain::names::types::Reference::Use {
                continue;
            }
            rows.write(Row {
                occurrence: node_cell(file, at),
                owner: scopes::owner(file, at),
                target: names::target(file, &mut words, at),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}
