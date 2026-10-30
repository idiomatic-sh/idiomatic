//! Builds the catalog adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The language catalog adapter, from what the edge passes.
#[must_use]
pub fn language() -> impl crate::outbound::catalog::interface::CatalogPort {
    super::adapters::language::adapter::CatalogLanguageAdapter
}
