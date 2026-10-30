//! JavaScript function facts from the `ESTree` projection.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `function` rows of one JavaScript file from its syntax tree.
pub(crate) struct FunctionItem;
ryo::reads! {FunctionItem => languages::Function |tree,at,symbols|{kinds:[Kind::Node(Node::FunctionDeclaration),Kind::Node(Node::FunctionExpression)] as kind_bits,
require (syntax::function_form(tree,at) != crate::domain::syntax::types::Declaredness::Expression);

row{id:node(at),name:value(syntax::function_name(tree,symbols,at)),file:file,span:span(at)}}}
