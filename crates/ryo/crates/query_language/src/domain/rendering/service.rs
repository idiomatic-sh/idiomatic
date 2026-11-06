//! The `rendering` service: the calls that read the physical text of the syntax tree (`render-syntax`, `fits-rendered`).

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::lowering::types::PlannedColumn;
use crate::domain::lowering::types::Sourced;
use crate::domain::lowering::types::Uniqueness;
use crate::domain::planning::service::push;
use crate::domain::resolving::service::argument_named;
use crate::domain::resolving::service::column_of;
use crate::domain::resolving::service::input_argument;
use crate::domain::resolving::service::position;
use crate::domain::resolving::service::positions_of;
use crate::domain::resolving::service::word_of;
use crate::domain::resolving::service::words_of;
use crate::domain::syntax::constants::ASSIGNMENT_TARGET;
use crate::domain::syntax::constants::ATTRIBUTE_RECEIVER;
use crate::domain::syntax::constants::POSITIONAL_ARGUMENT;
use crate::domain::syntax::constants::slot_name;
use crate::domain::syntax::types::ArgumentSlot;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Vocabulary;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::ArgumentValue;
use ryo_values::domain::program::types::Call;
use ryo_values::domain::program::types::QuerySpelling;
use ryo_values::domain::program::types::StepName;

fn semantic_arguments(call: &Call, allowed: &[ArgumentSlot]) -> Result<(), LoweringError> {
    for (at, argument) in call.arguments.iter().enumerate() {
        if !allowed
            .iter()
            .any(|slot| slot_name(*slot) == argument.name.0.as_ref())
            || call.arguments[..at]
                .iter()
                .any(|earlier| earlier.name == argument.name)
        {
            return Err(LoweringError::SemanticArgument {
                name: argument.name.0.to_string(),
                span: call.span,
            });
        }
    }
    Ok(())
}

fn semantic_source(builder: &mut Builder<'_>, call: &Call) -> Result<Sourced, LoweringError> {
    let sourced = input_argument(builder, call, ArgumentSlot::From)?;
    if crate::domain::planning::service::scoped(builder, sourced.input).scope
        != ryo_values::domain::plan_parts::types::StepScope::PerFile
    {
        return Err(LoweringError::CrossFileSemantic { span: call.span });
    }
    Ok(sourced)
}

/// Append exact syntax text only for rows that already survived structural selection.
pub(crate) fn call_render_syntax(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    semantic_arguments(
        call,
        &[
            ArgumentSlot::From,
            ArgumentSlot::Node,
            ArgumentSlot::Context,
            ArgumentSlot::Output,
        ],
    )?;
    let Sourced {
        input: from,
        mut fields,
    } = semantic_source(builder, call)?;
    let node = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Node)?.0),
        call.span,
    )?;
    if column_of(&fields, node).kind != CellKind::Node {
        return Err(LoweringError::KindMismatch { span: call.span });
    }
    let context = match word_of(call, ArgumentSlot::Context)?.0.as_ref() {
        ASSIGNMENT_TARGET => RenderContext::AssignmentTarget,
        ATTRIBUTE_RECEIVER => RenderContext::AttributeReceiver,
        POSITIONAL_ARGUMENT => RenderContext::PositionalArgument,
        _ => return Err(LoweringError::InvalidSemanticLiteral { span: call.span }),
    };
    let output = word_of(call, ArgumentSlot::Output)?;
    if fields.iter().any(|field| field.name == output) {
        return Err(LoweringError::DuplicateOutput {
            name: output.0.to_string(),
            span: call.span,
        });
    }
    fields.push(PlannedColumn {
        name: output,
        kind: CellKind::Symbol,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        Operation::RenderSyntax {
            from,
            node,
            context,
        },
        fields,
    );
    Ok(())
}

/// Keep rendered candidates whose physical statement replacement fits the declared width.
pub(crate) fn call_fits_rendered(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    semantic_arguments(
        call,
        &[
            ArgumentSlot::From,
            ArgumentSlot::Texts,
            ArgumentSlot::Literals,
            ArgumentSlot::Span,
            ArgumentSlot::MaxLine,
            ArgumentSlot::TabWidth,
        ],
    )?;
    let Sourced {
        input: from,
        fields,
    } = semantic_source(builder, call)?;
    let texts = positions_of(builder, &fields, &words_of(call, ArgumentSlot::Texts)?)?;
    let literals = words_of(call, ArgumentSlot::Literals)?;
    if texts.is_empty() || literals.len() != texts.len() + 1 {
        return Err(LoweringError::InvalidSemanticLiteral { span: call.span });
    }
    let span = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Span)?.0),
        call.span,
    )?;
    if texts
        .iter()
        .any(|text| column_of(&fields, *text).kind != CellKind::Symbol)
        || column_of(&fields, span).kind != CellKind::Span
    {
        return Err(LoweringError::KindMismatch { span: call.span });
    }
    let ArgumentValue::Number(max_line) = argument_named(call, ArgumentSlot::MaxLine)? else {
        return Err(LoweringError::InvalidSemanticLiteral { span: call.span });
    };
    let ArgumentValue::Number(tab_width) = argument_named(call, ArgumentSlot::TabWidth)? else {
        return Err(LoweringError::InvalidSemanticLiteral { span: call.span });
    };
    let Ok(tab_width) = u32::try_from(tab_width.0) else {
        return Err(LoweringError::InvalidSemanticLiteral { span: call.span });
    };
    if max_line.0 <= 0 || tab_width == 0 {
        return Err(LoweringError::InvalidSemanticLiteral { span: call.span });
    }
    push(
        builder,
        Some(name),
        Operation::FitsRendered {
            from,
            texts,
            literals: literals
                .into_iter()
                .map(|literal| LiteralText(literal.0))
                .collect(),
            span,
            max_line: ryo_values::domain::plans::types::MaxLine(max_line.0),
            tab_width: ryo_values::domain::plans::types::TabWidth(tab_width),
        },
        fields,
    );
    Ok(())
}
