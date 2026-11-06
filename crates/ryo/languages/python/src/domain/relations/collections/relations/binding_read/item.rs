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
pub(crate) struct BindingReadItem;
ryo::reads! {
 BindingReadItem => languages::BindingRead, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when indexed(tree, model, symbols) {
   None=>{partial INCOMPLETE => languages::Missing::Any;}
   Some(index)=>{
    each (index.reads.iter()) as answer {
     given context = languages::intern_derived(symbols, tree.source, vocabulary::context(answer.context));
given state = languages::intern_derived(symbols, tree.source, vocabulary::state(answer.state));
given since = vocabulary::availability(answer.state);
row { reference:node(answer.reference),name:value(answer.name),scope:node(answer.scope),phase:node(answer.phase),context:value(context),state:value(state),since_major:value(since.0),since_minor:value(since.1),file:file,span:span(answer.reference), }
    }
   }
  }
 }
}
