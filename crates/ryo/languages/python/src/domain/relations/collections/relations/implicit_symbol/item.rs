//! Emit the provider catalog once per physical module, not per embedded expression.
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    qualification::{constants::BUILTIN_NAMES, service::builtin_named},
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// Canonical Python builtin inventory; no module globals or fabricated syntax IDs.
pub(crate) struct ImplicitSymbolItem;
ryo::reads! {
 ImplicitSymbolItem => languages::ImplicitSymbol, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  each (BUILTIN_NAMES.iter()) as builtin {
   given since = builtin_named(*builtin)?;
   given name = languages::intern_derived(symbols, tree.source, *builtin);
   row {name:value(name),since_major:value(since.major),since_minor:value(since.minor),file:file,}
  }
 }
}
