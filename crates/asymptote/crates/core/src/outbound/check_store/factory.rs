//! Builds the check store's adapter from where the edge keeps it.

use super::types::Store;

/// The store adapter over `store`, which names its own file in the store's directory.
#[must_use]
pub fn postcard(store: Store) -> impl super::interface::CheckStorePort {
    super::adapters::postcard::adapter::CheckStorePostcardAdapter::new(store)
}
