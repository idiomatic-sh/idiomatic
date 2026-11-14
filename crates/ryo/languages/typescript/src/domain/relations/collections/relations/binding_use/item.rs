//! TypeScript `binding_use` rows: each identifier reference the parser's semantic pass resolves, once for each declaration of the name
//! it resolves to that the file's `defines` rows hold.
use crate::domain::bindings::service as bindings;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `binding_use` rows of one TypeScript file from its syntax tree.
pub(crate) struct BindingUseItem;
ryo::reads! { BindingUseItem => languages::BindingUse, model(ReadIndexes) |tree,model,at,symbols| {
 kinds: [Kind::Node(Node::Identifier),Kind::Node(Node::JSXIdentifier)] as kind_bits,
 given context = bindings::context(tree,symbols,at)?;
 given index = bindings::indexed(tree,&model.bindings);
 given candidates = bindings::candidates(tree,&index,at);
 given certainty = bindings::certainty(tree,symbols,candidates);
 each (candidates.iter()) as candidate {
 row { reference: node(at), binding: node(candidate.at), scope: node(scopes::enclosing(tree,at)), phase: node(bindings::phase(tree,at)), context: value(context), certainty: value(certainty), file: file, span: span(at) }
 }
} }
