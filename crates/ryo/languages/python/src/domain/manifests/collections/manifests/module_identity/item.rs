//! Emit only real requested file identities; unavailable root context is explicitly partial.
use crate::domain::{links::types::Linked, module_context::types::ModuleContexts};
use ryo::languages;
/// Project module identity reader.
pub(crate) struct ModuleIdentityItem;
ryo::reads! {
    ModuleIdentityItem => languages::ModuleIdentity, manifests(Linked) |input, symbols|
    model contexts: ModuleContexts = (Some(input.derived.module_context.clone())) {
        each (contexts.partial.then_some("module identity requires a named root or external package context")) as reason {
            partial reason => languages::Missing::Any;
        }
        each (&contexts.rows) as module {
            row {
                file: value(module.file),
                path: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&module.path))),
            }
        }
    }
}
