//! The `compile` service: local free-function helpers, private to this use case unless it is an SDK contract.

use super::errors::CompileError;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::validation::errors::ValidationError;

/// Says a refusal of the validation as the use case's error.
pub(super) fn invalid(error: ValidationError) -> CompileError {
    match error {
        ValidationError::UnknownRelation { name, span } => {
            CompileError::UnknownRelation { name, span }
        }
        ValidationError::UnknownStep { span, .. }
        | ValidationError::UnknownParameter { span, .. }
        | ValidationError::UnknownOperation { span, .. }
        | ValidationError::MissingArgument { span, .. }
        | ValidationError::DuplicateStep { span, .. }
        | ValidationError::JoinNeedsColumns { span } => CompileError::Invalid {
            span,
            message: error.to_string(),
        },
    }
}

/// Says a refusal of the lowering as the use case's error.
pub(super) fn unresolved(error: LoweringError) -> CompileError {
    match error {
        LoweringError::UnknownRelation { name, span } => {
            CompileError::UnknownRelation { name, span }
        }
        LoweringError::KindMismatch { span }
        | LoweringError::UnknownWord { span, .. }
        | LoweringError::NotText { span, .. } => CompileError::Type {
            span,
            message: error.to_string(),
        },
        LoweringError::UnknownStep { span, .. }
        | LoweringError::DuplicateOutput { span, .. }
        | LoweringError::InvalidRegex { span, .. }
        | LoweringError::UnknownColumn { span, .. }
        | LoweringError::UnknownSide { span, .. }
        | LoweringError::UnknownSideColumn { span, .. }
        | LoweringError::UnknownParameterType { span, .. }
        | LoweringError::FunctionArity { span, .. }
        | LoweringError::BadArgument { span, .. } => CompileError::Invalid {
            span,
            message: error.to_string(),
        },
    }
}
