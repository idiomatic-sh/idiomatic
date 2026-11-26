//! An immutable module registry of a query. Retaining it keeps its identity valid while roots from the same catalog are admitted.
use crate::resources::query::RyoQlQuery;

/// The modules of a query, by source.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ModuleSources(std::sync::Arc<std::collections::BTreeMap<
    ryo_values::domain::query_tokens::types::QuerySourceId,
    ryo_values::domain::query_tokens::types::QueryDocumentSource,
>>);

impl ModuleSources {
    pub(super) fn identity(&self) -> usize { std::sync::Arc::as_ptr(&self.0) as usize }

    pub(super) fn sources(&self) -> impl Iterator<Item = (&str, &std::sync::Arc<str>)> {
        self.0.iter().map(|(id, document)| (id.0.as_ref(), &document.text.0))
    }
}

/// The modules of `query`; none when it has none.
pub(super) fn query_modules(query: &RyoQlQuery) -> Option<ModuleSources> {
    let modules = query.clone().into_bundle().modules;
    (!modules.is_empty()).then_some(ModuleSources(modules))
}
