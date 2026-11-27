//! What the `typecheck` use case promises its callers.

use super::errors::TypecheckError;
use super::types::Checked;
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::query_requests::types::QueryRequest;

/// What `typecheck` offers its callers.
pub trait TypecheckUseCase: Send + Sync {
    /// Checks that RyoQL text is correct: well formed, reading only relations ryo owns, and well typed.
    /// It needs no language linked; the relations are ryo's own.
    ///
    /// # Errors
    ///
    /// `Syntax`, `UnknownRelation`, `Type` or `Invalid`, with the place where the text is wrong.
    fn typecheck(&self, request: QueryRequest) -> Result<ValidateReport, TypecheckError>;

    /// The checks alone, over the same text: the plan they built, or the query they hand on to the plan that runs.
    ///
    /// # Errors
    ///
    /// What `typecheck` refuses the text for, but for what the plan that runs judges.
    fn check(&self, request: QueryRequest) -> Result<Checked, TypecheckError>;
}
