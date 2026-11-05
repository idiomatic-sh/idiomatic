//! JavaScript reference rows: each name read where it occurs, a static member chain as one dotted name.
use crate::domain::names::service as names;
use crate::domain::names::types::Reference;
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `reference` rows of one JavaScript file from its syntax tree.
pub(crate) struct ReferenceItem;
ryo::reads! {ReferenceItem => languages::Reference |tree,at,symbols|{kinds:[Kind::Node(JavaScriptNode::Identifier),Kind::Node(JavaScriptNode::MemberExpression)] as kind_bits,
require (names::referenced(tree,at) == Reference::Use);
row{occurrence:node(at),owner:value(scopes::owner(tree,at)),target:value(syntax::target(tree,symbols,at)),file:file,span:span(at)}}}
