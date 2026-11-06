//! Type and pattern validation for the replacement projection.

use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Vocabulary;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::{Call, QuerySpelling, StepName};

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, PlannedColumn, Sourced, Uniqueness};
use crate::domain::planning::service::push;
use crate::domain::resolving::service::{
    argument_named, column_of, input_argument, position, positions_of, word_of, words_of,
};
use crate::domain::syntax::constants::{
    ASSIGNMENT_TARGET, ATTRIBUTE_RECEIVER, POSITIONAL_ARGUMENT, slot_name,
};
use crate::domain::syntax::types::ArgumentSlot;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_values::domain::program::types::ArgumentValue;

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

/// Search a checked physical source span and append each absolute occurrence span.
pub(crate) fn call_source_matches(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    for (at, argument) in call.arguments.iter().enumerate() {
        if ![
            ArgumentSlot::From,
            ArgumentSlot::Span,
            ArgumentSlot::Pattern,
            ArgumentSlot::Output,
            ArgumentSlot::BaseLine,
            ArgumentSlot::LineOutput,
            ArgumentSlot::CaptureName,
            ArgumentSlot::CaptureOutput,
            ArgumentSlot::CaptureTextOutput,
            ArgumentSlot::CaptureLineOutput,
        ]
        .iter()
        .any(|slot| slot_name(*slot) == argument.name.0.as_ref())
            || call
                .arguments
                .iter()
                .take(at)
                .any(|earlier| earlier.name == argument.name)
        {
            return Err(LoweringError::SourceMatchesArgument {
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
        return Err(LoweringError::CrossFileSourceMatches { span: call.span });
    }
    let span = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Span)?.0),
        call.span,
    )?;
    if column_of(&fields, span).kind != CellKind::Span {
        return Err(LoweringError::KindMismatch { span: call.span });
    }
    let pattern = LiteralText(word_of(call, ArgumentSlot::Pattern)?.0);
    let output = word_of(call, ArgumentSlot::Output)?;
    let has_base_line = call
        .arguments
        .iter()
        .any(|argument| argument.name.0.as_ref() == slot_name(ArgumentSlot::BaseLine));
    let has_line_output = call
        .arguments
        .iter()
        .any(|argument| argument.name.0.as_ref() == slot_name(ArgumentSlot::LineOutput));
    if has_base_line != has_line_output {
        return Err(LoweringError::SourceMatchesLinePair { span: call.span });
    }
    let base_line = if has_base_line {
        let at = position(
            &fields,
            QuerySpelling(&word_of(call, ArgumentSlot::BaseLine)?.0),
            call.span,
        )?;
        if column_of(&fields, at).kind != CellKind::Number {
            return Err(LoweringError::KindMismatch { span: call.span });
        }
        Some(at)
    } else {
        None
    };
    let line_output = if has_line_output {
        Some(word_of(call, ArgumentSlot::LineOutput)?)
    } else {
        None
    };
    let present = |slot: ArgumentSlot| {
        call.arguments
            .iter()
            .any(|argument| argument.name.0.as_ref() == slot_name(slot))
    };
    let capture = present(ArgumentSlot::CaptureName);
    let capture_output = present(ArgumentSlot::CaptureOutput);
    let capture_text = present(ArgumentSlot::CaptureTextOutput);
    let capture_line = present(ArgumentSlot::CaptureLineOutput);
    if capture != capture_output
        || (capture_text || capture_line) && !capture
        || capture_line && !has_base_line
    {
        return Err(LoweringError::SourceMatchesCapturePair { span: call.span });
    }
    let capture_name = capture
        .then(|| word_of(call, ArgumentSlot::CaptureName))
        .transpose()?;
    let capture_span_output = capture_output
        .then(|| word_of(call, ArgumentSlot::CaptureOutput))
        .transpose()?;
    let capture_text_output = capture_text
        .then(|| word_of(call, ArgumentSlot::CaptureTextOutput))
        .transpose()?;
    let capture_line_output = capture_line
        .then(|| word_of(call, ArgumentSlot::CaptureLineOutput))
        .transpose()?;
    let outputs = [
        Some(&output),
        line_output.as_ref(),
        capture_span_output.as_ref(),
        capture_text_output.as_ref(),
        capture_line_output.as_ref(),
    ];
    for (index, name) in outputs
        .iter()
        .enumerate()
        .filter_map(|(index, output)| output.map(|name| (index, name)))
    {
        if fields.iter().any(|field| &field.name == name)
            || outputs[..index]
                .iter()
                .flatten()
                .any(|earlier| *earlier == name)
        {
            return Err(LoweringError::DuplicateOutput {
                name: name.0.to_string(),
                span: call.span,
            });
        }
    }
    let compiled = regex_automata::meta::Regex::new(&pattern.0).map_err(|error| {
        LoweringError::InvalidRegex {
            reason: error.to_string(),
            span: call.span,
        }
    })?;
    if let Some(name) = &capture_name
        && compiled
            .group_info()
            .to_index(regex_automata::util::primitives::PatternID::ZERO, &name.0)
            .is_none()
    {
        return Err(LoweringError::SourceMatchesCaptureMissing {
            name: name.0.to_string(),
            span: call.span,
        });
    }
    fields.push(PlannedColumn {
        name: output,
        kind: CellKind::Span,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    if let Some(line_output) = line_output {
        fields.push(PlannedColumn {
            name: line_output,
            kind: CellKind::Number,
            unique: Uniqueness::Shared,
            words: Vocabulary::Open,
        });
    }
    for (name, kind) in [
        (capture_span_output, CellKind::Span),
        (capture_text_output, CellKind::Symbol),
        (capture_line_output, CellKind::Number),
    ] {
        if let Some(name) = name {
            fields.push(PlannedColumn {
                name,
                kind,
                unique: Uniqueness::Shared,
                words: Vocabulary::Open,
            });
        }
    }
    push(
        builder,
        Some(name),
        Operation::SourceMatches {
            from,
            span,
            base_line,
            pattern,
            capture: capture_name.map(|name| LiteralText(name.0)),
            capture_text: if capture_text {
                ryo_values::domain::plans::types::TextOutput::Appended
            } else {
                ryo_values::domain::plans::types::TextOutput::Omitted
            },
            capture_line: if capture_line {
                ryo_values::domain::plans::types::LineOutput::Appended
            } else {
                ryo_values::domain::plans::types::LineOutput::Omitted
            },
        },
        fields,
    );
    Ok(())
}

/// Validate constant regex and append a new text column without changing input identity.
pub(crate) fn call_replace(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let text = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Text)?.0),
        call.span,
    )?;
    if column_of(&fields, text).kind != CellKind::Symbol
        && column_of(&fields, text).kind != CellKind::File
    {
        return Err(LoweringError::KindMismatch { span: call.span });
    }
    let pattern = LiteralText(word_of(call, ArgumentSlot::Pattern)?.0);
    let replacement = LiteralText(word_of(call, ArgumentSlot::Replacement)?.0);
    let output = word_of(call, ArgumentSlot::Output)?;
    if fields.iter().any(|field| field.name == output) {
        return Err(LoweringError::DuplicateOutput {
            name: output.0.to_string(),
            span: call.span,
        });
    }
    let _compiled = regex_automata::meta::Regex::new(&pattern.0).map_err(|error| {
        LoweringError::InvalidRegex {
            reason: error.to_string(),
            span: call.span,
        }
    })?;
    fields.push(PlannedColumn {
        name: output,
        kind: CellKind::Symbol,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        Operation::Replace {
            from,
            text,
            pattern,
            replacement,
        },
        fields,
    );
    Ok(())
}
