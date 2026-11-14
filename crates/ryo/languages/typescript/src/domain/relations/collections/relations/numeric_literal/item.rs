//! TypeScript `numeric_literal` rows: the kind and value the parser decoded for each numeric literal, never its spelling.
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{NUMERIC_KIND, NUMERIC_VALUE};
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `numeric-literal` rows of one TypeScript file from the numeric properties of its literals.
pub(crate) struct NumericLiteralItem;
ryo::reads! {
 NumericLiteralItem => languages::NumericLiteral, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::Literal)] as kind_bits,
 given kind = languages::property_at(tree.elements, at, NUMERIC_KIND)?;
 given value = languages::property_at(tree.elements, at, NUMERIC_VALUE)?;

 row { id: node(at), kind: value(languages::intern_derived(symbols, tree.source, kind)), value: value(languages::intern_derived(symbols, tree.source, value)), exact_digits: value(crate::domain::syntax::service::exactness(tree, at)), file: file, span: span(at) }

 }
}
