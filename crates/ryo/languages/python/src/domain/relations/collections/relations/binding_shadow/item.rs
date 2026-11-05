//! Emit generic static association facts once from the module root.
use crate::domain::{
    binding_analysis::service::indexed,
    binding_results::{constants::INCOMPLETE, service as vocabulary},
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// Static binding observations with original source identities.
pub(crate) struct BindingShadowItem;
ryo::reads! {
 BindingShadowItem => languages::BindingShadow, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when indexed(tree, model, symbols) {
   None=>{partial INCOMPLETE => languages::Missing::Any;}
   Some(index)=>{
    each (index.shadows.iter()) as answer {
     given relationship = languages::intern_derived(symbols, tree.source, languages::NameText(vocabulary::relationship(answer.relationship)));
row { binding:node(answer.binding),previous:node(answer.previous),scope:node(answer.scope),previous_scope:node(answer.previous_scope),phase:node(answer.phase),relationship:value(relationship),file:file, }
    }
   }
  }
 }
}
