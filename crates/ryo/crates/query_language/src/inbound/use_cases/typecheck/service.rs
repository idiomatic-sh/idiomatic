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
        | ValidationError::DuplicateObservation { span, .. }
        | ValidationError::BlankObservation { span }
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
        | LoweringError::OrderNotNumber { span, .. }
        | LoweringError::UnknownWord { span, .. }
        | LoweringError::NotText { span, .. } => TypecheckError::Type {
            span,
            message: error.to_string(),
        },
        LoweringError::SpanBoundsArgument { span, .. }
        | LoweringError::SemanticArgument { span, .. }
        | LoweringError::CrossFileSemantic { span }
        | LoweringError::InvalidSemanticLiteral { span, .. }
        | LoweringError::SourceMatchesArgument { span, .. }
        | LoweringError::SourceMatchesCapturePair { span, .. }
        | LoweringError::SourceMatchesCaptureMissing { span, .. }
        | LoweringError::CrossFileSourceMatches { span }
        | LoweringError::SourceMatchesLinePair { span }
        | LoweringError::CrossFileSpan { span }
        | LoweringError::CrossFileSpanKey { span }
        | LoweringError::MixedVertices { span }
        | LoweringError::SpanArgument { span, .. }
        | LoweringError::UnknownStep { span, .. }
        | LoweringError::DuplicateOutput { span, .. }
        | LoweringError::WrittenRowWidth { span, .. }
        | LoweringError::InvalidRegex { span, .. }
        | LoweringError::UnknownColumn { span, .. }
        | LoweringError::ComputedColumn { span }
        | LoweringError::NegatedCondition { span }
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

/// What the typechecker said of a text it refused, as the core says it: the kind, the message, the place and where it was imported from.
#[must_use]
pub fn refused(mut error: TypecheckError) -> ryo_values::domain::program::types::RefusedQuery {
    use ryo_relations::domain::relations::types::{Offset, Span};
    use ryo_values::domain::program::types::{RefusalMessage, RefusedKind, RefusedQuery};
    use ryo_values::domain::query_tokens::types::{Attribution, ByteOffset, QuerySpan};
    let offset = |at: ByteOffset| Offset(u32::try_from(at.0).unwrap_or(u32::MAX));
    let span = |at: QuerySpan| Span { start: offset(at.start), end: offset(at.end) };
    let nowhere = Span { start: Offset(0), end: Offset(0) };
    let mut origin = Attribution::Unlocated;
    let (kind, message, span) = loop {
        match error {
            TypecheckError::Located { origin: found, error: inner } => {
                origin = Attribution::Located(found);
                error = *inner;
            }
            TypecheckError::Syntax { span: at, message } => break (RefusedKind::Syntax, message, span(at)),
            TypecheckError::UnknownRelation { name, span: at } => break (RefusedKind::UnknownRelation, name, span(at)),
            TypecheckError::Type { span: at, message } => break (RefusedKind::Type, message, span(at)),
            TypecheckError::Invalid { span: at, message } => break (RefusedKind::Invalid, message, span(at)),
            TypecheckError::ObservationInputs { message } => break (RefusedKind::Invalid, message, nowhere),
            TypecheckError::Catalog { reason } => break (RefusedKind::Catalog, reason, nowhere),
            TypecheckError::UnknownKind { reason } => break (RefusedKind::Invalid, reason, nowhere),
        }
    };
    RefusedQuery { kind, message: RefusalMessage(message), span, origin }
}
