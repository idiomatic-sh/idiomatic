//! JavaScript `static_import` facts from the `ESTree` projection: every import declaration, every dynamic `import(..)` and every call of
//! `require`, whose binding is unknown and whose module is unknown unless a string names it.
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `static-import` rows of one JavaScript file from its syntax tree.
pub(crate) struct StaticImportItem;
ryo::reads! {StaticImportItem => languages::StaticImport |tree,at,symbols|{kinds:[Kind::Node(JavaScriptNode::ImportDeclaration),Kind::Node(JavaScriptNode::ImportExpression),Kind::Node(JavaScriptNode::CallExpression)] as kind_bits,
given target = syntax::import_target(tree,symbols,at)?; each (syntax::imports(tree,at)) as binding {
row{occurrence:node(at),owner:value(crate::domain::scopes::service::owner(tree,at)),binding:value(syntax::import_binding(tree,symbols,binding)),target:value(target),file:file,span:span(at)}}}}
