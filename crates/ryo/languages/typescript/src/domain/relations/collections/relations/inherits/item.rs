//! TypeScript inherits rows.
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `inherits` rows of one TypeScript file from its syntax tree.
pub(crate) struct InheritsItem;
ryo::reads! { InheritsItem => languages::Inherits, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::ClassDeclaration),Kind::Node(Node::ClassExpression),Kind::Node(Node::TSInterfaceDeclaration)] as kind_bits,
 each (names::bases(tree,at)) as base {
 row { occurrence: node(base),declaration: node(at),target: value(names::target(tree,symbols,base)),file: file,span: span(base) }
 }
} }
