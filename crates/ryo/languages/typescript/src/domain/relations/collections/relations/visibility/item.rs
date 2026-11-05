//! TypeScript visibility rows.
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use crate::domain::visibilities::service as visibilities;
use ryo::languages;

/// Reads the `visibility` rows of one TypeScript file from its syntax tree.
pub(crate) struct VisibilityItem;
ryo::reads! { VisibilityItem => languages::Visibility |tree,at,symbols| {
 kinds: [Kind::Node(TypeScriptNode::ExportNamedDeclaration),Kind::Node(TypeScriptNode::ExportDefaultDeclaration),Kind::Node(TypeScriptNode::MethodDefinition),Kind::Node(TypeScriptNode::PropertyDefinition),Kind::Node(TypeScriptNode::AccessorProperty)] as kind_bits,
 each (visibilities::visible(tree,at)) as visible {
 row { occurrence: node(visible.occurrence),declaration: node(visible.declaration),text: value(visibilities::written(tree,symbols,&visible)),file: file,span: value(visibilities::span(&visible)) }
 }
} }
