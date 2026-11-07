//! JavaScript scope rows: each lexical environment, under the one around it.
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::scopes::types::Scope;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `scope` rows of one JavaScript file from its syntax tree.
pub(crate) struct ScopeItem;
ryo::reads! {ScopeItem => languages::Scope |tree,at,_symbols|{kinds:[Kind::Node(Node::Program),Kind::Node(Node::FunctionDeclaration),Kind::Node(Node::FunctionExpression),Kind::Node(Node::ArrowFunctionExpression),Kind::Node(Node::ClassDeclaration),Kind::Node(Node::ClassExpression),Kind::Node(Node::BlockStatement),Kind::Node(Node::CatchClause),Kind::Node(Node::ForStatement),Kind::Node(Node::ForInStatement),Kind::Node(Node::ForOfStatement),Kind::Node(Node::StaticBlock)] as kind_bits,
require (scopes::opens(syntax::kind(tree,at)) == Scope::Opens);
row{parent:node(scopes::enclosing(tree,at)),child:node(at),file:file,span:span(at)}}}
