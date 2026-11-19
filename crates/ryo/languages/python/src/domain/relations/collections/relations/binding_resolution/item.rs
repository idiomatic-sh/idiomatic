//! Emit schedule-order name resolution once from the module root.
use crate::domain::{
    binding_resolution::service::resolved,
    binding_results::{constants::INCOMPLETE, service as vocabulary},
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// For each read or deletion, the event in effect in each namespace on its lookup path.
pub(crate) struct BindingResolutionItem;
ryo::reads! {
 BindingResolutionItem => languages::BindingResolution, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when resolved(tree, model, symbols) {
   None=>{partial INCOMPLETE => languages::Missing::Any;}
   Some(scratch)=>{
    each (scratch.resolutions.iter()) as answer {
     given kind = languages::intern_derived(symbols, tree.source, vocabulary::event(answer.kind));
given context = languages::intern_derived(symbols, tree.source, vocabulary::context(answer.context));
row { reference:node(answer.reference),event:node(answer.event),kind:value(kind),scope:node(answer.scope),lexical:node(answer.lexical),depth:value(answer.depth.0),ordinal:value(answer.ordinal),context:value(context),file:file,span:span(answer.reference), }
    }
   }
  }
 }
}
