//! What the `compile` use case promises its callers.

use super::errors::CompileError;
use super::types::CompileReport;
use ryo_values::domain::plans::types::QueryRequest;

/// What `compile` offers its callers.
pub trait CompileUseCase: Send + Sync {
    /// Compiles RyoQL text into the plan the engine walks. The same text gives the same plan.
    ///
    /// # Errors
    ///
    /// Whatever `validate` refuses the text for.
    fn compile(&self, request: QueryRequest) -> Result<CompileReport, CompileError>;
}
