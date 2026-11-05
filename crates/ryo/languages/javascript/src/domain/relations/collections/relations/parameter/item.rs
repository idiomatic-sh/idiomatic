//! JavaScript parameter facts from the `ESTree` projection.
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `parameter` rows of one JavaScript file from its syntax tree.
pub(crate) struct ParameterItem;
ryo::reads! {ParameterItem => languages::Parameter |tree,at,symbols|{kinds:[Kind::Node(JavaScriptNode::FunctionDeclaration),Kind::Node(JavaScriptNode::FunctionExpression),Kind::Node(JavaScriptNode::ArrowFunctionExpression)] as kind_bits,
each (syntax::parameters(tree,at)) as (index,parameter) {
row{id:node(parameter),kind:value(if syntax::kind(tree,parameter)==Kind::Node(JavaScriptNode::RestElement){languages::ParameterKind::VariadicPositional}else{languages::ParameterKind::Positional}),refinement:value(syntax::kind_symbol(tree,symbols,parameter)),name:value(syntax::parameter_name(tree,symbols,parameter)),ordinal:value(languages::Integer(i64::from(index.0))),owner:node(at),file:file,span:span(parameter)}}}}
