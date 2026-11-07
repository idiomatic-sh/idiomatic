//! Builds the `check_store` adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

/// The postcard store of `store`, under the build that runs; under `-n/--dry-run` it writes nothing.
#[must_use]
pub fn postcard(
    store: &crate::domain::settings::types::Store,
) -> impl super::interface::CheckStorePort + use<> {
    super::adapters::postcard::adapter::CheckStorePostcardAdapter::at(store)
}
