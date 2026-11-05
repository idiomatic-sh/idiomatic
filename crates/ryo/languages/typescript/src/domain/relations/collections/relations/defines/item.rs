//! TypeScript defines rows.
use crate::domain::names::service as names;
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `defines` rows of one TypeScript file from its syntax tree.
pub(crate) struct DefinesItem;
ryo::reads! { DefinesItem => languages::Defines |tree,at,symbols| {
 kinds: [Kind::Node(TypeScriptNode::FunctionDeclaration),Kind::Node(TypeScriptNode::TSDeclareFunction),Kind::Node(TypeScriptNode::ClassDeclaration),Kind::Node(TypeScriptNode::TSInterfaceDeclaration),Kind::Node(TypeScriptNode::TSTypeAliasDeclaration),Kind::Node(TypeScriptNode::TSTypeParameter),Kind::Node(TypeScriptNode::TSEnumDeclaration),Kind::Node(TypeScriptNode::TSEnumMember),Kind::Node(TypeScriptNode::TSModuleDeclaration),Kind::Node(TypeScriptNode::VariableDeclarator),Kind::Node(TypeScriptNode::ImportSpecifier),Kind::Node(TypeScriptNode::ImportDefaultSpecifier),Kind::Node(TypeScriptNode::ImportNamespaceSpecifier),Kind::Node(TypeScriptNode::MethodDefinition),Kind::Node(TypeScriptNode::TSMethodSignature),Kind::Node(TypeScriptNode::PropertyDefinition),Kind::Node(TypeScriptNode::AccessorProperty),Kind::Node(TypeScriptNode::TSPropertySignature),Kind::Node(TypeScriptNode::LabeledStatement)] as kind_bits,
 each (names::declared(tree,at)) as binding {
 row { id: node(at),kind: value(names::declaration_kind(tree,at)),refinement: value(crate::domain::syntax::service::kind_symbol(tree,symbols,at)),name: value(names::identifier(tree,symbols,binding)),owner: node(scopes::enclosing(tree,at)),file: file,span: span(binding) }
 }
} }
