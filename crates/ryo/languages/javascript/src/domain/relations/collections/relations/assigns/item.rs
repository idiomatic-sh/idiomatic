//! JavaScript assigns rows: a name bound to a string literal, by a declarator or an assignment.
use crate::domain::names::service as names;
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `assigns` rows of one JavaScript file from its syntax tree.
pub(crate) struct AssignsItem;
ryo::reads! {AssignsItem => languages::Assigns |tree,at,_symbols|{kinds:[Kind::Node(JavaScriptNode::VariableDeclarator),Kind::Node(JavaScriptNode::AssignmentExpression)] as kind_bits,
given assigned = names::assignment(tree,at)?;
row{occurrence:node(at),binding:node(assigned.binding),value:node(assigned.value),owner:value(scopes::owner(tree,at)),file:file,span:span(assigned.value)}}}
