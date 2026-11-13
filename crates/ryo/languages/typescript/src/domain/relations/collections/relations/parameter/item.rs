//! TypeScript parameter rows.
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `parameter` rows of one TypeScript file from its syntax tree.
pub(crate) struct ParameterItem;
ryo::reads! { ParameterItem => languages::Parameter, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::FunctionDeclaration),Kind::Node(Node::FunctionExpression),Kind::Node(Node::ArrowFunctionExpression),Kind::Node(Node::TSDeclareFunction),Kind::Node(Node::TSEmptyBodyFunctionExpression)] as kind_bits,
 each (names::parameters(tree,at)) as (ordinal,parameter) {
 row { id: node(parameter), kind: value(crate::domain::syntax::service::parameter_kind(tree,parameter)), refinement: value(crate::domain::syntax::service::kind_symbol(tree,symbols,parameter)), name: value(names::parameter_name(tree,symbols,parameter)), ordinal: value(languages::Integer(i64::from(ordinal.0))), owner: node(at), file: file, span: span(parameter) }
 }
} }
