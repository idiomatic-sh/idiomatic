//! TypeScript `static_import` rows from its standard syntax tree: every import declaration, every dynamic `import(..)` and every call of
//! `require`, whose binding is unknown and whose module is unknown unless a string names it.
use crate::domain::imports::service as imports;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `static-import` rows of one TypeScript file from its syntax tree.
pub(crate) struct StaticImportItem;
ryo::reads! {
 StaticImportItem => languages::StaticImport, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::ImportDeclaration), Kind::Node(Node::ImportExpression), Kind::Node(Node::CallExpression)] as kind_bits,
 given target = imports::import_source(tree,symbols,at)?;
 each (imports::imports(tree,at)) as binding {
 row { occurrence: node(at), owner: value(scopes::owner(tree,at)), binding: value(imports::import_name(tree,symbols,binding)), target: value(target), file: file, span: span(at) }
}
 }
}
