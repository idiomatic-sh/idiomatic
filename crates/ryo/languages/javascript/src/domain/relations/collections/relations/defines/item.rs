//! JavaScript defines rows: each name a declaration binds, with its kind and the environment it is bound in.
use crate::domain::names::service as names;
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `defines` rows of one JavaScript file from its syntax tree.
pub(crate) struct DefinesItem;
ryo::reads! {DefinesItem => languages::Defines |tree,at,symbols|{kinds:[Kind::Node(JavaScriptNode::FunctionDeclaration),Kind::Node(JavaScriptNode::ClassDeclaration),Kind::Node(JavaScriptNode::VariableDeclarator),Kind::Node(JavaScriptNode::ImportSpecifier),Kind::Node(JavaScriptNode::ImportDefaultSpecifier),Kind::Node(JavaScriptNode::ImportNamespaceSpecifier),Kind::Node(JavaScriptNode::MethodDefinition),Kind::Node(JavaScriptNode::PropertyDefinition),Kind::Node(JavaScriptNode::LabeledStatement)] as kind_bits,
each (names::declared(tree,at)) as binding {
row{id:node(at),kind:value(names::declaration_kind(tree,at)),refinement:value(syntax::kind_symbol(tree,symbols,at)),name:value(syntax::named(tree,symbols,binding)),owner:node(scopes::enclosing(tree,at)),file:file,span:span(binding)}
}}}
