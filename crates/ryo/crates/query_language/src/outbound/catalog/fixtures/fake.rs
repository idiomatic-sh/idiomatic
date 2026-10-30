//! The fake of the `catalog` port: what the core's tests stand in for it with.

use ryo_language::domain::relations::types::Relation;

use super::super::errors::CatalogError;
use super::super::interface::CatalogPort;
use super::super::types::Relations;

/// What the core's tests stand in for `catalog` with: it lists the relations it is given, or fails.
#[derive(Debug)]
pub struct CatalogFake {
    /// The relations it lists.
    pub relations: Vec<Relation>,
    /// Whether it fails instead of listing.
    pub fails: bool,
}

impl Default for CatalogFake {
    /// A catalog holding the `function` relation.
    fn default() -> Self {
        Self {
            relations: vec![Relation::Function],
            fails: false,
        }
    }
}

impl CatalogPort for CatalogFake {
    fn relations(&self) -> Result<Relations, CatalogError> {
        if self.fails {
            return Err(CatalogError::Unavailable {
                reason: "the fake was told to fail".to_owned(),
            });
        }
        Ok(Relations(self.relations.clone()))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
