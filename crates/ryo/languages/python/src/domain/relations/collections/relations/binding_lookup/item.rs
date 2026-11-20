//! Emit the schedule-order lookup once from the module root.
use crate::domain::{
    binding_in_effect::service::looked_up,
    binding_results::{constants::INCOMPLETE, service as vocabulary},
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// For each read or deletion and each namespace on its lookup path, the binding in effect and each later event of the name.
pub(crate) struct BindingLookupItem;
ryo::reads! {
 BindingLookupItem => languages::BindingLookup, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when looked_up(tree, model, symbols) {
   None=>{partial INCOMPLETE => languages::Missing::Any;}
   Some(scratch)=>{
    each (scratch.rows.iter()) as answer {
     given kind = languages::intern_derived(symbols, tree.source, vocabulary::event(answer.kind));
given context = languages::intern_derived(symbols, tree.source, vocabulary::context(answer.context));
row { reference:node(answer.reference),name:value(answer.name),event:node(answer.event),kind:value(kind),scope:node(answer.scope),lexical:node(answer.lexical),depth:value(answer.depth),ordinal:value(answer.ordinal),context:value(context),file:file,span:span(answer.reference), }
    }
   }
  }
 }
}
