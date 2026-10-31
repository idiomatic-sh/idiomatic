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
pub(crate) struct BindingUseItem;
ryo::reads! {
 BindingUseItem => languages::BindingUse, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when indexed(tree, model, symbols) {
   None=>{partial INCOMPLETE => languages::Missing::Any;}
   Some(index)=>{
    each (index.uses.iter()) as answer {
     given context = languages::intern_derived(symbols, tree.source, languages::NameText(vocabulary::context(answer.context)));
given certainty = languages::intern_derived(symbols, tree.source, languages::NameText(vocabulary::certainty(answer.certainty)));
row { reference:node(answer.reference),binding:node(answer.binding),scope:node(answer.scope),phase:node(answer.phase),context:value(context),certainty:value(certainty),file:file,span:span(answer.reference), }
    }
   }
  }
 }
}
