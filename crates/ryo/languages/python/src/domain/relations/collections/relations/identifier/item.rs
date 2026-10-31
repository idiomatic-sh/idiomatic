//! Emit parser-normalized spelling without changing lossless source tokens.
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{
        constants::IDENTIFIERS_INCOMPLETE,
        service::{identifier_text, identifiers, kind_bits},
        types::Kind,
    },
};
use ryo::languages;
/// Python lexical identifier occurrences.
pub(crate) struct IdentifierItem;
ryo::reads! {
 IdentifierItem => languages::Identifier, model(Declarations) |tree, model, at, symbols| {
 kinds:[Kind::Node(Node::Module)] as kind_bits,
 when identifiers(tree) {
  Some(occurrences)=>{
   each (occurrences) as identifier {
    given name = languages::intern_derived(symbols,tree.source,languages::NameText(identifier_text(tree,identifier).0));
    row {id:node(identifier),name:value(name),file:file,span:span(identifier),}
   }
  }
  None=>{partial IDENTIFIERS_INCOMPLETE => languages::Missing::Any;}
 }
 }
}
