//! The `spanning` service: the calls that compose or project the byte spans of rows (`span-between`, `span-bounds`).

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::lowering::types::PlannedColumn;
use crate::domain::lowering::types::Sourced;
use crate::domain::lowering::types::Uniqueness;
use crate::domain::planning::service::push;
use crate::domain::resolving::service::column_of;
use crate::domain::resolving::service::input_argument;
use crate::domain::resolving::service::position;
use crate::domain::resolving::service::word_of;
use crate::domain::syntax::constants::slot_name;
use crate::domain::syntax::types::ArgumentSlot;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Vocabulary;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::Call;
use ryo_values::domain::program::types::QuerySpelling;
use ryo_values::domain::program::types::StepName;

/// Compose byte offsets only after fragment inputs have resolved to concrete scopes.
pub(crate) fn call_span_between(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    for (at, argument) in call.arguments.iter().enumerate() {
        if ![
            ArgumentSlot::From,
            ArgumentSlot::Start,
            ArgumentSlot::End,
            ArgumentSlot::Output,
        ]
        .iter()
        .any(|slot| slot_name(*slot) == argument.name.0.as_ref())
            || call
                .arguments
                .iter()
                .take(at)
                .any(|earlier| earlier.name == argument.name)
        {
            return Err(LoweringError::SpanArgument {
                name: argument.name.0.to_string(),
                span: call.span,
            });
        }
    }
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    if crate::domain::planning::service::scoped(builder, from).scope
        != ryo_values::domain::plan_parts::types::StepScope::PerFile
    {
        return Err(LoweringError::CrossFileSpan { span: call.span });
    }
    let start = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Start)?.0),
        call.span,
    )?;
    let end = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::End)?.0),
        call.span,
    )?;
    if column_of(&fields, start).kind != CellKind::Span
        || column_of(&fields, end).kind != CellKind::Span
    {
        return Err(LoweringError::KindMismatch { span: call.span });
    }
    let output = word_of(call, ArgumentSlot::Output)?;
    if fields.iter().any(|field| field.name == output) {
        return Err(LoweringError::DuplicateOutput {
            name: output.0.to_string(),
            span: call.span,
        });
    }
    fields.push(PlannedColumn {
        name: output,
        kind: CellKind::Span,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        Operation::SpanBetween { from, start, end },
        fields,
    );
    Ok(())
}

/// Project checked physical byte endpoints without changing input row identity.
pub(crate) fn call_span_bounds(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    for (at, argument) in call.arguments.iter().enumerate() {
        if ![
            ArgumentSlot::From,
            ArgumentSlot::Span,
            ArgumentSlot::StartOutput,
            ArgumentSlot::EndOutput,
        ]
        .iter()
        .any(|slot| slot_name(*slot) == argument.name.0.as_ref())
            || call
                .arguments
                .iter()
                .take(at)
                .any(|earlier| earlier.name == argument.name)
        {
            return Err(LoweringError::SpanBoundsArgument {
                name: argument.name.0.to_string(),
                span: call.span,
            });
        }
    }
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let span = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Span)?.0),
        call.span,
    )?;
    if column_of(&fields, span).kind != CellKind::Span {
        return Err(LoweringError::KindMismatch { span: call.span });
    }
    let start = word_of(call, ArgumentSlot::StartOutput)?;
    let end = word_of(call, ArgumentSlot::EndOutput)?;
    for output in [&start, &end] {
        if fields.iter().any(|field| &field.name == output) {
            return Err(LoweringError::DuplicateOutput {
                name: output.0.to_string(),
                span: call.span,
            });
        }
    }
    if start == end {
        return Err(LoweringError::DuplicateOutput {
            name: end.0.to_string(),
            span: call.span,
        });
    }
    for output in [start, end] {
        fields.push(PlannedColumn {
            name: output,
            kind: CellKind::Number,
            unique: Uniqueness::Shared,
            words: Vocabulary::Open,
        });
    }
    push(
        builder,
        Some(name),
        Operation::SpanBounds { from, span },
        fields,
    );
    Ok(())
}
