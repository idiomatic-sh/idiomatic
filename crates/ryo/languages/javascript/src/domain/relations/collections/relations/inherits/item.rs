//! JavaScript inherits rows: what a class extends.
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `inherits` rows of one JavaScript file from its syntax tree.
pub(crate) struct InheritsItem;
ryo::reads! {InheritsItem => languages::Inherits |tree,at,symbols|{kinds:[Kind::Node(Node::ClassDeclaration),Kind::Node(Node::ClassExpression)] as kind_bits,
each (names::bases(tree,at)) as base {
row{occurrence:node(base),declaration:node(at),target:value(syntax::target(tree,symbols,base)),file:file,span:span(base)}
}}}
