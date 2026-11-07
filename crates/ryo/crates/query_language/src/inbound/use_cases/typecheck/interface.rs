//! What the `typecheck` use case promises its callers.

use super::errors::TypecheckError;
use ryo_values::domain::plans::types::QueryRequest;
use ryo_values::domain::descriptions::types::ValidateReport;

/// What `typecheck` offers its callers.
pub trait TypecheckUseCase: Send + Sync {
    /// Checks that RyoQL text is correct: well formed, reading only relations ryo owns, and well typed.
    /// It needs no language linked; the relations are ryo's own.
    ///
    /// # Errors
    ///
    /// `Syntax`, `UnknownRelation`, `Type` or `Invalid`, with the place where the text is wrong.
    fn typecheck(&self, request: QueryRequest) -> Result<ValidateReport, TypecheckError>;
}
