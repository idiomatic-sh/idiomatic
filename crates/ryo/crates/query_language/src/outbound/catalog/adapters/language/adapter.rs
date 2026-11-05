//! The catalog port, implemented by the `language` crate's catalog of relations.

use ryo_relations::domain::relations::service::all;

use crate::outbound::catalog::errors::CatalogError;
use crate::outbound::catalog::interface::CatalogPort;
use crate::outbound::catalog::types::Relations;

/// The language catalog adapter.
#[derive(Debug)]
pub(crate) struct CatalogLanguageAdapter;

impl CatalogPort for CatalogLanguageAdapter {
    fn relations(&self) -> Result<Relations, CatalogError> {
        Ok(Relations(all().to_vec()))
    }
}
