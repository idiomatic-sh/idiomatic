//! The module sources a query carries, for the executor that runs a round of queries: two queries that share a catalog of modules
//! share one registry, which the executor keeps under the registry's identity.

use crate::domain::query::types::RyoQlQuery;

/// An immutable registry of the module sources of a query's catalog. Keeping it keeps its identity valid while the queries of the
/// same catalog are admitted.
#[doc(hidden)]
#[derive(Clone, PartialEq, Eq)]
pub struct ModuleSources(
    std::sync::Arc<
        std::collections::BTreeMap<
            ryo_values::domain::query_tokens::types::QuerySourceId,
            ryo_values::domain::query_tokens::types::QueryDocumentSource,
        >,
    >,
);

impl ModuleSources {
    /// The registry's identity: the address of its shared map, equal for the clones of one registry.
    #[must_use]
    pub fn identity(&self) -> usize {
        std::sync::Arc::as_ptr(&self.0).addr()
    }

    /// The sources, each by the name of its module and its text.
    pub fn sources(&self) -> impl Iterator<Item = (&str, &std::sync::Arc<str>)> {
        self.0.iter().map(|(id, document)| (id.0.as_ref(), &document.text.0))
    }
}

/// The module sources of `query`'s catalog; nothing for a query with none.
#[doc(hidden)]
#[must_use]
pub fn query_modules(query: &RyoQlQuery) -> Option<ModuleSources> {
    let modules = query.clone().into_bundle().modules;
    (!modules.is_empty()).then_some(ModuleSources(modules))
}
