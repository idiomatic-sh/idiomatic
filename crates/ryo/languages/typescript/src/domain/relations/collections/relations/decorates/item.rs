//! TypeScript decorates rows.
use crate::domain::names::service as names;
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::service::parent;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `decorates` rows of one TypeScript file from its syntax tree.
pub(crate) struct DecoratesItem;
ryo::reads! { DecoratesItem => languages::Decorates |tree,at,symbols| {
 kinds: [Kind::Node(TypeScriptNode::Decorator)] as kind_bits,
 given declaration = parent(tree, at);
 row { occurrence: node(at),declaration: node(declaration),ordinal: value(languages::Integer(i64::try_from(names::nodes(tree,declaration).take_while(|child|*child!=at).filter(|child|kind(tree,*child)==Kind::Node(TypeScriptNode::Decorator)).count()).unwrap_or(i64::MAX))),target: value(names::target(tree,symbols,at)),arguments: value(crate::domain::decorations::service::arguments(tree,symbols,at)),file: file,span: span(at) }

} }
