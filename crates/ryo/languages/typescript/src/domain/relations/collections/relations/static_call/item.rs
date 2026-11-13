//! TypeScript `static_call` rows.
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `static-call` rows of one TypeScript file from its syntax tree.
pub(crate) struct StaticCallItem;
ryo::reads! { StaticCallItem => languages::StaticCall, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::CallExpression),Kind::Node(Node::NewExpression)] as kind_bits,
 given callee = names::nodes(tree,at).next()?;
 row { occurrence: node(at),owner: value(scopes::owner(tree,at)),target: value(names::target(tree,symbols,callee)),file: file,span: span(callee) }

} }
