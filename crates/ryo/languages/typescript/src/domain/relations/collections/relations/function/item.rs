//! TypeScript function rows from its standard syntax tree.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `function` rows of one TypeScript file from its syntax tree.
pub(crate) struct FunctionItem;
ryo::reads! {
 FunctionItem => languages::Function, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::FunctionDeclaration), Kind::Node(Node::FunctionExpression), Kind::Node(Node::TSDeclareFunction), Kind::Node(Node::TSEmptyBodyFunctionExpression)] as kind_bits,
 require (scopes::function_form(tree, at) != crate::domain::scopes::types::Declaredness::Expression);
 row { id: node(at), name: value(scopes::function_name(tree,symbols,at)), file: file, span: span(at) }
 }
}

// The engine fills `function` from the walked file: every row of the file, once.
ryo::fills! {
    FunctionItem => languages::Function, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Function as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[
            Kind::Node(Node::FunctionDeclaration),
            Kind::Node(Node::FunctionExpression),
            Kind::Node(Node::TSDeclareFunction),
            Kind::Node(Node::TSEmptyBodyFunctionExpression),
        ]) {
            if scopes::function_form(file, at) == crate::domain::scopes::types::Declaredness::Expression {
                continue;
            }
            rows.write(Row {
                id: node_cell(file, at),
                name: scopes::function_name(file, &mut words, at),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}
