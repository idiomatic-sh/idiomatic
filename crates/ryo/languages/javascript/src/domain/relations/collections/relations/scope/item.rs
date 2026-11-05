//! JavaScript scope rows: each lexical environment, under the one around it.
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::scopes::service as scopes;
use crate::domain::scopes::types::Scope;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `scope` rows of one JavaScript file from its syntax tree.
pub(crate) struct ScopeItem;
ryo::reads! {ScopeItem => languages::Scope |tree,at,_symbols|{kinds:[Kind::Node(JavaScriptNode::Program),Kind::Node(JavaScriptNode::FunctionDeclaration),Kind::Node(JavaScriptNode::FunctionExpression),Kind::Node(JavaScriptNode::ArrowFunctionExpression),Kind::Node(JavaScriptNode::ClassDeclaration),Kind::Node(JavaScriptNode::ClassExpression),Kind::Node(JavaScriptNode::BlockStatement),Kind::Node(JavaScriptNode::CatchClause),Kind::Node(JavaScriptNode::ForStatement),Kind::Node(JavaScriptNode::ForInStatement),Kind::Node(JavaScriptNode::ForOfStatement),Kind::Node(JavaScriptNode::StaticBlock)] as kind_bits,
require (scopes::opens(syntax::kind(tree,at)) == Scope::Opens);
row{parent:node(scopes::enclosing(tree,at)),child:node(at),file:file,span:span(at)}}}
