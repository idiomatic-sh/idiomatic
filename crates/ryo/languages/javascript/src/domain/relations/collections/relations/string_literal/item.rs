//! JavaScript `string_literal` facts from the `ESTree` projection.
use crate::domain::fact_notes::service::specifier;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `string-literal` rows of one JavaScript file from its syntax tree.
pub(crate) struct StringLiteralItem;
ryo::reads! {StringLiteralItem => languages::StringLiteral |tree,at,symbols|{kinds:[Kind::Node(Node::Literal)] as kind_bits,
given decoded = syntax::string(tree,symbols,at)?;
row{occurrence:node(at),owner:value(crate::domain::scopes::service::owner(tree,at)),value:value(decoded),file:file,span:span(at),import:value(specifier(tree,at))}}}
