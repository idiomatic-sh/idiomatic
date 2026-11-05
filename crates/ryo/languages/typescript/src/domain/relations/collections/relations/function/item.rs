//! TypeScript function rows from its standard syntax tree.
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `function` rows of one TypeScript file from its syntax tree.
pub(crate) struct FunctionItem;
ryo::reads! {
 FunctionItem => languages::Function |tree, at, symbols| {
 kinds: [Kind::Node(Node::FunctionDeclaration), Kind::Node(Node::FunctionExpression), Kind::Node(Node::TSDeclareFunction), Kind::Node(Node::TSEmptyBodyFunctionExpression)] as kind_bits,
 require (scopes::function_form(tree, at) != crate::domain::scopes::types::Declaredness::Expression);
 row { id: node(at), name: value(scopes::function_name(tree,symbols,at)), file: file, span: span(at) }
 }
}
