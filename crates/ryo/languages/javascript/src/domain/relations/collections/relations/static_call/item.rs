//! JavaScript `static_call` facts from the `ESTree` projection.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `static-call` rows of one JavaScript file from its syntax tree.
pub(crate) struct StaticCallItem;
ryo::reads! {StaticCallItem => languages::StaticCall |tree,at,symbols|{kinds:[Kind::Node(Node::CallExpression),Kind::Node(Node::NewExpression)] as kind_bits,
given callee = syntax::nodes(tree,at).next()?;
row{occurrence:node(at),owner:value(crate::domain::scopes::service::owner(tree,at)),target:value(syntax::target(tree,symbols,callee)),file:file,span:span(callee)}}}
