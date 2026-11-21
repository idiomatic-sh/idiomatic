//! Emit the lookup paths once from the module root.
use crate::domain::{
    binding_results::{constants::INCOMPLETE, service as vocabulary},
    lookup_path::service::paths,
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// For each read or deletion, the namespaces in which Python looks the name up, nearest first.
pub(crate) struct LookupScopeItem;
ryo::reads! {
 LookupScopeItem => languages::LookupScope, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when paths(tree, model, symbols) {
   None=>{partial INCOMPLETE => languages::Missing::Any;}
   Some(scratch)=>{
    each (scratch.rows.iter()) as answer {
     given context = languages::intern_derived(symbols, tree.source, vocabulary::context(answer.context));
row { reference:node(answer.reference),name:value(answer.name),scope:node(answer.scope),depth:value(answer.depth),context:value(context),file:file,span:span(answer.reference), }
    }
   }
  }
 }
}
