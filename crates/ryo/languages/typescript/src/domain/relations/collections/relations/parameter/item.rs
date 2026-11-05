//! TypeScript parameter rows.
use crate::domain::names::service as names;
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `parameter` rows of one TypeScript file from its syntax tree.
pub(crate) struct ParameterItem;
ryo::reads! { ParameterItem => languages::Parameter |tree,at,symbols| {
 kinds: [Kind::Node(TypeScriptNode::FunctionDeclaration),Kind::Node(TypeScriptNode::FunctionExpression),Kind::Node(TypeScriptNode::ArrowFunctionExpression),Kind::Node(TypeScriptNode::TSDeclareFunction),Kind::Node(TypeScriptNode::TSEmptyBodyFunctionExpression)] as kind_bits,
 each (names::parameters(tree,at)) as (ordinal,parameter) {
 row { id: node(parameter), kind: value(crate::domain::syntax::service::parameter_kind(tree,parameter)), refinement: value(crate::domain::syntax::service::kind_symbol(tree,symbols,parameter)), name: value(names::parameter_name(tree,symbols,parameter)), ordinal: value(languages::Integer(i64::from(ordinal.0))), owner: node(at), file: file, span: span(parameter) }
 }
} }
