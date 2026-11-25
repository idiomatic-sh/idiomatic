//! TypeScript `binding_use` rows: each identifier reference the parser's semantic pass resolves, once for each declaration of the name
//! it resolves to that the file's `defines` rows hold.
use crate::domain::bindings::service as bindings;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `binding_use` rows of one TypeScript file from its syntax tree.
pub(crate) struct BindingUseItem;
ryo::reads! { BindingUseItem => languages::BindingUse, model(ReadIndexes) |tree,model,at,symbols| {
 kinds: [Kind::Node(Node::Identifier),Kind::Node(Node::JSXIdentifier)] as kind_bits,
 given context = bindings::context(tree,symbols,at)?;
 given index = bindings::indexed(tree,&model.bindings);
 given candidates = bindings::candidates(tree,&index,at);
 given certainty = bindings::certainty(tree,symbols,candidates);
 each (candidates.iter()) as candidate {
 row { reference: node(at), binding: node(candidate.at), scope: node(scopes::enclosing(tree,at)), phase: node(bindings::phase(tree,at)), context: value(context), certainty: value(certainty), file: file, span: span(at) }
 }
} }

// The engine fills `binding-use` from the walked file: every row of the file, once.
ryo::fills! {
    BindingUseItem => languages::BindingUse, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::BindingUse as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::Identifier), Kind::Node(Node::JSXIdentifier)]) {
            let Some(context) = bindings::context(file, &mut words, at) else {
                continue;
            };
            let index = bindings::indexed(file, &file.model.bindings);
            let candidates = bindings::candidates(file, &index, at);
            let certainty = bindings::certainty(file, &mut words, candidates);
            for candidate in candidates {
                rows.write(Row {
                    reference: node_cell(file, at),
                    binding: node_cell(file, candidate.at),
                    scope: node_cell(file, scopes::enclosing(file, at)),
                    phase: node_cell(file, bindings::phase(file, at)),
                    context,
                    certainty,
                    file: file.file,
                    span: span_at(file, at),
                });
            }
        }
    }
}
