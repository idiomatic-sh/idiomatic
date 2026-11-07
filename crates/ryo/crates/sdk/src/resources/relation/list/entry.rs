//! `relation list`: composes, calls the service once.

use ryo_core::inbound::use_cases::list_relations::interface::ListRelationsUseCase as _;

use super::errors::ListError;
use super::schemas;
use super::types::ListReport;

/// `relation list`: lists every relation ryo owns.
///
/// # Errors
///
/// `Language` when the catalog cannot be read.
pub fn list() -> Result<ListReport, ListError> {
    super::composition::build()
        .list_relations()
        .map(schemas::report)
        .map_err(schemas::error)
}
