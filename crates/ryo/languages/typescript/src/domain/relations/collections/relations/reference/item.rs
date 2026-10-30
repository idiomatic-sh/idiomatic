//! TypeScript reference rows.
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `reference` rows of one TypeScript file from its syntax tree.
pub(crate) struct ReferenceItem;
ryo::reads! { ReferenceItem => languages::Reference |tree,at,symbols| {
 kinds: [Kind::Node(Node::Identifier),Kind::Node(Node::MemberExpression),Kind::Node(Node::TSQualifiedName)] as kind_bits,
 require names::referenced(tree,at) == crate::domain::names::types::Reference::Use;
 row { occurrence: node(at),owner: value(scopes::owner(tree,at)),target: value(names::target(tree,symbols,at)),file: file,span: span(at) }

} }
