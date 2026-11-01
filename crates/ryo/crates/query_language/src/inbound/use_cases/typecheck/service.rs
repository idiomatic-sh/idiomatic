//! The `typecheck` service: local free-function helpers, private to this use case unless it is an SDK contract.

use super::errors::TypecheckError;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::validation::errors::ValidationError;

/// Says a refusal of the validation as the use case's error.
pub(super) fn invalid(error: ValidationError) -> TypecheckError {
    match error {
        ValidationError::Located { origin, error } => TypecheckError::Located {
            origin,
            error: Box::new(invalid(*error)),
        },
        ValidationError::UnknownRelation { name, span } => {
            TypecheckError::UnknownRelation { name, span }
        }
        ValidationError::UnknownStep { span, .. }
        | ValidationError::UnknownParameter { span, .. }
        | ValidationError::UnknownOperation { span, .. }
        | ValidationError::MissingArgument { span, .. }
        | ValidationError::DuplicateStep { span, .. }
        | ValidationError::JoinNeedsColumns { span } => TypecheckError::Invalid {
            span,
            message: error.to_string(),
        },
    }
}

/// Says a refusal of the lowering as the use case's error.
pub(super) fn unresolved(error: LoweringError) -> TypecheckError {
    match error {
        LoweringError::Located { origin, error } => TypecheckError::Located {
            origin,
            error: Box::new(unresolved(*error)),
        },
        LoweringError::FragmentContract { origin, message } => {
            let span = origin.span;
            TypecheckError::Located {
                origin,
                error: Box::new(TypecheckError::Invalid { span, message }),
            }
        }
        LoweringError::UnknownRelation { name, span } => {
            TypecheckError::UnknownRelation { name, span }
        }
        LoweringError::KindMismatch { span }
        | LoweringError::UnknownWord { span, .. }
        | LoweringError::NotText { span, .. } => TypecheckError::Type {
            span,
            message: error.to_string(),
        },
        LoweringError::CrossFileSpan { span }
        | LoweringError::SpanArgument { span, .. }
        | LoweringError::UnknownStep { span, .. }
        | LoweringError::DuplicateOutput { span, .. }
        | LoweringError::InvalidRegex { span, .. }
        | LoweringError::UnknownColumn { span, .. }
        | LoweringError::UnknownSide { span, .. }
        | LoweringError::UnknownSideColumn { span, .. }
        | LoweringError::UnknownParameterType { span, .. }
        | LoweringError::FunctionArity { span, .. }
        | LoweringError::TooManyComparisonOrigins { span }
        | LoweringError::BadArgument { span, .. } => TypecheckError::Invalid {
            span,
            message: error.to_string(),
        },
    }
}
