//! TypeScript `string_literal` rows from its standard syntax tree.
use crate::domain::fact_notes::service::specifier;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `string-literal` rows of one TypeScript file from its syntax tree.
pub(crate) struct StringLiteralItem;
ryo::reads! {
 StringLiteralItem => languages::StringLiteral, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::Literal)] as kind_bits,
 given value = names::string(tree,symbols,at)?;

 row { occurrence: node(at), owner: value(scopes::owner(tree,at)), value: value(value), file: file, span: span(at), import: value(specifier(tree,at)) }

 }
}
