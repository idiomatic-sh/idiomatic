//! `operator list`: composes, calls the service once.

use ryo_core::inbound::use_cases::list_operators::interface::ListOperatorsUseCase as _;

use super::errors::ListError;
use super::schemas;
use super::types::ListReport;

/// `operator list`: lists every operator in documentation order.
///
/// # Errors
///
/// Never fails.
pub fn list() -> Result<ListReport, ListError> {
    super::composition::build()
        .list_operators()
        .map(schemas::report)
        .map_err(schemas::error)
}
