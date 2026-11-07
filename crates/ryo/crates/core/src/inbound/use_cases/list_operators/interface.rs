//! What the `list_operators` use case promises its callers.

use super::errors::ListOperatorsError;
use super::types::ListOperatorsReport;

/// What `list_operators` offers its callers.
pub trait ListOperatorsUseCase: Send + Sync {
    /// Lists every operator in documentation order.
    ///
    /// # Errors
    ///
    /// Never fails.
    fn list_operators(&self) -> Result<ListOperatorsReport, ListOperatorsError>;
}
