//! TypeScript assigns rows.
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `assigns` rows of one TypeScript file from its syntax tree.
pub(crate) struct AssignsItem;
ryo::reads! { AssignsItem => languages::Assigns, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::VariableDeclarator),Kind::Node(Node::AssignmentExpression)] as kind_bits,
 given assigned = names::assignment(tree,at)?;
 row { occurrence: node(at),binding: node(assigned.binding),value: node(assigned.value),owner: value(scopes::owner(tree,at)),file: file,span: span(assigned.value) }

} }
