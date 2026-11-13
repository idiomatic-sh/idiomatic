//! TypeScript annotates rows.
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::parent;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `annotates` rows of one TypeScript file from its syntax tree.
pub(crate) struct AnnotatesItem;
ryo::reads! { AnnotatesItem => languages::Annotates, model(ReadIndexes) |tree,_model,at,symbols| {
 kinds: [Kind::Node(Node::TSTypeAnnotation)] as kind_bits,
 given annotation = names::nodes(tree,at).next()?;
 row { occurrence: node(at),owner: node(parent(tree, at)),annotation: value(names::written(tree,symbols,annotation)),file: file,span: span(annotation) }

} }
