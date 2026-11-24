/// An immutable module registry from the linked release. Retaining it keeps its
/// identity valid while roots from the same catalog are admitted.
#[derive(Clone, PartialEq, Eq)]
pub struct ModuleSources(std::sync::Arc<std::collections::BTreeMap<
    ryo_values::domain::query_tokens::types::QuerySourceId,
    ryo_values::domain::query_tokens::types::QueryDocumentSource,
>>);

impl ModuleSources {
    pub fn identity(&self) -> usize { std::sync::Arc::as_ptr(&self.0) as usize }

    pub fn sources(&self) -> impl Iterator<Item = (&str, &std::sync::Arc<str>)> {
        self.0.iter().map(|(id, document)| (id.0.as_ref(), &document.text.0))
    }
}

pub fn query_modules(query: &crate::query::RyoQlQuery) -> Option<ModuleSources> {
    let modules = query.clone().into_bundle().modules;
    (!modules.is_empty()).then_some(ModuleSources(modules))
}
