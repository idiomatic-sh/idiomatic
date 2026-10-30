//! What the `catalog` port asks of the outside: the relations ryo owns.

use super::errors::CatalogError;
use super::types::Relations;

/// What every `catalog` adapter implements.
pub trait CatalogPort: Send + Sync {
    /// Lists every relation ryo owns, with no language linked.
    ///
    /// # Errors
    ///
    /// `Unavailable` when the catalog cannot be read.
    fn relations(&self) -> Result<Relations, CatalogError>;
}
