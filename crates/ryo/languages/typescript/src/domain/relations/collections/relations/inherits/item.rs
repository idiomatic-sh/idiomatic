//! TypeScript inherits rows.
use crate::domain::names::service as names;
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `inherits` rows of one TypeScript file from its syntax tree.
pub(crate) struct InheritsItem;
ryo::reads! { InheritsItem => languages::Inherits |tree,at,symbols| {
 kinds: [Kind::Node(TypeScriptNode::ClassDeclaration),Kind::Node(TypeScriptNode::ClassExpression),Kind::Node(TypeScriptNode::TSInterfaceDeclaration)] as kind_bits,
 each (names::bases(tree,at)) as base {
 row { occurrence: node(base),declaration: node(at),target: value(names::target(tree,symbols,base)),file: file,span: span(base) }
 }
} }
