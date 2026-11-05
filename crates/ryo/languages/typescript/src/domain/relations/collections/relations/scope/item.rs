//! TypeScript scope rows from its standard syntax tree.
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `scope` rows of one TypeScript file from its syntax tree.
pub(crate) struct ScopeItem;
ryo::reads! {
 ScopeItem => languages::Scope |tree, at, symbols| {
 kinds: [Kind::Node(TypeScriptNode::Program),Kind::Node(TypeScriptNode::FunctionDeclaration),Kind::Node(TypeScriptNode::FunctionExpression),Kind::Node(TypeScriptNode::ArrowFunctionExpression),Kind::Node(TypeScriptNode::ClassDeclaration),Kind::Node(TypeScriptNode::ClassExpression),Kind::Node(TypeScriptNode::TSDeclareFunction),Kind::Node(TypeScriptNode::TSEmptyBodyFunctionExpression),Kind::Node(TypeScriptNode::TSModuleDeclaration),Kind::Node(TypeScriptNode::TSModuleBlock),Kind::Node(TypeScriptNode::BlockStatement),Kind::Node(TypeScriptNode::CatchClause),Kind::Node(TypeScriptNode::ForStatement),Kind::Node(TypeScriptNode::ForInStatement),Kind::Node(TypeScriptNode::ForOfStatement),Kind::Node(TypeScriptNode::StaticBlock)] as kind_bits,
 require scopes::opens(crate::domain::syntax::service::kind(tree,at)) == crate::domain::scopes::types::Scope::Opens;

 row { parent: node(scopes::enclosing(tree,at)), child: node(at), file: file, span: span(at) }

 }
}
