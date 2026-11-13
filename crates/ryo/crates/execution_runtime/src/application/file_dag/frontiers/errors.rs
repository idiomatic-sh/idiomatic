//! The `frontiers` concept's failures.
use crate::outbound::facts::errors::FactsError;
use ryo_execution_domain::domain::operating::errors::OperatingError;

/// Why the frontiers of one rule could not be prepared.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::frontiers::errors::FrontiersError;
/// fn reason(error: &FrontiersError) -> String {
///     error.to_string()
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum FrontiersError {
    #[error(transparent)]
    Facts(#[from] FactsError),
    #[error(transparent)]
    Metadata(#[from] OperatingError),
}
