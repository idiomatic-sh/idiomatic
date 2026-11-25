//! TypeScript defines rows.
use crate::domain::bindings::service::definitions;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `defines` rows of one TypeScript file from its syntax tree.
pub(crate) struct DefinesItem;
ryo::reads! { DefinesItem => languages::Defines, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::FunctionDeclaration),Kind::Node(Node::TSDeclareFunction),Kind::Node(Node::ClassDeclaration),Kind::Node(Node::TSInterfaceDeclaration),Kind::Node(Node::TSTypeAliasDeclaration),Kind::Node(Node::TSTypeParameter),Kind::Node(Node::TSEnumDeclaration),Kind::Node(Node::TSEnumMember),Kind::Node(Node::TSModuleDeclaration),Kind::Node(Node::VariableDeclarator),Kind::Node(Node::ImportSpecifier),Kind::Node(Node::ImportDefaultSpecifier),Kind::Node(Node::ImportNamespaceSpecifier),Kind::Node(Node::MethodDefinition),Kind::Node(Node::TSMethodSignature),Kind::Node(Node::PropertyDefinition),Kind::Node(Node::AccessorProperty),Kind::Node(Node::TSPropertySignature),Kind::Node(Node::LabeledStatement),Kind::Node(Node::FunctionExpression),Kind::Node(Node::ArrowFunctionExpression),Kind::Node(Node::TSEmptyBodyFunctionExpression),Kind::Node(Node::ClassExpression),Kind::Node(Node::CatchClause)] as kind_bits,
 each (definitions(tree,at)) as definition {
 row { id: node(definition.id),kind: value(definition.kind),refinement: value(crate::domain::syntax::service::kind_symbol(tree,symbols,definition.id)),name: value(names::identifier(tree,symbols,definition.binding)),owner: node(definition.owner),file: file,span: span(definition.binding) }
 }
} }

// The engine fills `defines` from the walked file: every row of the file, once.
ryo::fills! {
    DefinesItem => languages::Defines, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Defines as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::FunctionDeclaration),Kind::Node(Node::TSDeclareFunction),Kind::Node(Node::ClassDeclaration),Kind::Node(Node::TSInterfaceDeclaration),Kind::Node(Node::TSTypeAliasDeclaration),Kind::Node(Node::TSTypeParameter),Kind::Node(Node::TSEnumDeclaration),Kind::Node(Node::TSEnumMember),Kind::Node(Node::TSModuleDeclaration),Kind::Node(Node::VariableDeclarator),Kind::Node(Node::ImportSpecifier),Kind::Node(Node::ImportDefaultSpecifier),Kind::Node(Node::ImportNamespaceSpecifier),Kind::Node(Node::MethodDefinition),Kind::Node(Node::TSMethodSignature),Kind::Node(Node::PropertyDefinition),Kind::Node(Node::AccessorProperty),Kind::Node(Node::TSPropertySignature),Kind::Node(Node::LabeledStatement),Kind::Node(Node::FunctionExpression),Kind::Node(Node::ArrowFunctionExpression),Kind::Node(Node::TSEmptyBodyFunctionExpression),Kind::Node(Node::ClassExpression),Kind::Node(Node::CatchClause)]) {
            for definition in definitions(file, at) {
                rows.write(Row {
                    id: node_cell(file, definition.id),
                    kind: definition.kind,
                    refinement: crate::domain::syntax::service::kind_symbol(file, &mut words, definition.id),
                    name: names::identifier(file, &mut words, definition.binding),
                    owner: node_cell(file, definition.owner),
                    file: file.file,
                    span: span_at(file, definition.binding),
                });
            }
        }
    }
}
