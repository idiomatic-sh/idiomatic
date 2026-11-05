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
pub(crate) struct BindingEventItem;
ryo::reads! {
 BindingEventItem => languages::BindingEvent, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when indexed(tree, model, symbols) {
   None=>{partial INCOMPLETE => languages::Missing::Any;}
   Some(index)=>{
    each (index.events.iter()) as answer {
     given kind = languages::intern_derived(symbols, tree.source, languages::NameText(vocabulary::event(answer.kind)));
row { occurrence:node(answer.occurrence),name:value(answer.name),scope:node(answer.scope),phase:node(answer.phase),kind:value(kind),ordinal:value(answer.ordinal),file:file,span:span(answer.occurrence), }
    }
   }
  }
 }
}
