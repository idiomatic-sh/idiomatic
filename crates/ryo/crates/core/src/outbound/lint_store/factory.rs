//! Builds the store adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The store kept in a file of the cache directory `store` names, for the build it names.
#[must_use]
pub fn postcard(
    store: super::types::Store,
) -> impl crate::outbound::lint_store::interface::LintStorePort {
    super::adapters::postcard::adapter::LintStorePostcardAdapter::new(store)
}

/// The store kept in memory, for the life of the adapter: a run that keeps nothing between processes.
#[must_use]
pub fn memory() -> impl crate::outbound::lint_store::interface::LintStorePort {
    super::adapters::memory::adapter::LintStoreMemoryAdapter::default()
}

/// No store: a run keeps nothing for the next, and makes nothing only a store reads.
#[must_use]
pub fn discard() -> impl crate::outbound::lint_store::interface::LintStorePort {
    super::adapters::discard::adapter::LintStoreDiscardAdapter
}
