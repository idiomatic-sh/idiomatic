//! TypeScript `static_call` rows.
use crate::domain::names::service as names;
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `static-call` rows of one TypeScript file from its syntax tree.
pub(crate) struct StaticCallItem;
ryo::reads! { StaticCallItem => languages::StaticCall |tree,at,symbols| {
 kinds: [Kind::Node(TypeScriptNode::CallExpression),Kind::Node(TypeScriptNode::NewExpression)] as kind_bits,
 given callee = names::nodes(tree,at).next()?;
 row { occurrence: node(at),owner: value(scopes::owner(tree,at)),target: value(names::target(tree,symbols,callee)),file: file,span: span(callee) }

} }
