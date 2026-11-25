//! TypeScript `binding_read` rows: each identifier reference, with how it uses its name and what the parser's semantic pass binds it
//! to: one declaration, a name declared more than once, a name the language provides, or nothing.
use crate::domain::bindings::service as bindings;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `binding_read` rows of one TypeScript file from its syntax tree.
pub(crate) struct BindingReadItem;
ryo::reads! { BindingReadItem => languages::BindingRead, model(ReadIndexes) |tree,model,at,symbols| {
 kinds: [Kind::Node(Node::Identifier),Kind::Node(Node::JSXIdentifier)] as kind_bits,
 given context = bindings::context(tree,symbols,at)?;
 given index = bindings::indexed(tree,&model.bindings);
 given state = bindings::state(tree,symbols,&index,at);
 row { reference: node(at), name: value(names::identifier(tree,symbols,at)), scope: node(scopes::enclosing(tree,at)), phase: node(bindings::phase(tree,at)), context: value(context), state: value(state), since_major: value(languages::Integer(0)), since_minor: value(languages::Integer(0)), file: file, span: span(at) }
} }

// The engine fills `binding-read` from the walked file: every row of the file, once.
ryo::fills! {
    BindingReadItem => languages::BindingRead, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::BindingRead as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::Identifier), Kind::Node(Node::JSXIdentifier)]) {
            let Some(context) = bindings::context(file, &mut words, at) else {
                continue;
            };
            let index = bindings::indexed(file, &file.model.bindings);
            let state = bindings::state(file, &mut words, &index, at);
            rows.write(Row {
                reference: node_cell(file, at),
                name: names::identifier(file, &mut words, at),
                scope: node_cell(file, scopes::enclosing(file, at)),
                phase: node_cell(file, bindings::phase(file, at)),
                context,
                state,
                since_major: languages::Integer(0),
                since_minor: languages::Integer(0),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}
