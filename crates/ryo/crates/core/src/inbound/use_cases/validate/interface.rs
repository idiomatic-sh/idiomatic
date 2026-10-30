//! What the `validate` use case promises its callers.

use super::errors::ValidateError;
use super::types::ValidateRequest;
use ryo_values::domain::descriptions::types::ValidateReport;

/// What `validate` offers its callers.
pub trait ValidateUseCase: Send + Sync {
    /// Checks that RyoQL text is correct, reading the relations ryo owns and the ones the host declares, and that every word it
    /// compares the kind of a node or a token with names a kind of a linked language, as a run checks it before it reads a file. With
    /// no language linked there is no kind to check against, and the words are not asked.
    ///
    /// # Errors
    ///
    /// `Refused` with the reason and the place when the text is not a correct query, `Uncompilable` when a correct query cannot be
    /// compiled, `UnknownKind` for a word that names no kind of a linked language.
    fn validate(&self, request: ValidateRequest) -> Result<ValidateReport, ValidateError>;
}
