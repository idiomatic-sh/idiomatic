//! Type and pattern validation for the replacement projection.

use ryo_language::domain::relations::types::CellKind;
use ryo_language::domain::terms::types::Vocabulary;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::{Call, QuerySpelling, StepName};

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, PlannedColumn, Sourced, Uniqueness};
use crate::domain::planning::service::push;
use crate::domain::resolving::service::{column_of, input_argument, position, word_of};
use crate::domain::syntax::types::ArgumentSlot;

/// Compose byte offsets only after fragment inputs have resolved to concrete scopes.
pub(crate) fn call_span_between(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    for (at, argument) in call.arguments.iter().enumerate() {
        if !matches!(
            argument.name.0.as_ref(),
            "from" | "start" | "end" | "output"
        ) || call
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
        != ryo_values::domain::plans::types::StepScope::PerFile
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
        if !matches!(
            argument.name.0.as_ref(),
            "from" | "span" | "start-output" | "end-output"
        ) || call
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
        if !matches!(
            argument.name.0.as_ref(),
            "from" | "span" | "pattern" | "output" | "base-line" | "line-output"
        ) || call
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
        != ryo_values::domain::plans::types::StepScope::PerFile
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
        .any(|argument| argument.name.0.as_ref() == "base-line");
    let has_line_output = call
        .arguments
        .iter()
        .any(|argument| argument.name.0.as_ref() == "line-output");
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
    if fields.iter().any(|field| field.name == output) {
        return Err(LoweringError::DuplicateOutput {
            name: output.0.to_string(),
            span: call.span,
        });
    }
    if let Some(line_output) = &line_output
        && (line_output == &output || fields.iter().any(|field| &field.name == line_output))
    {
        return Err(LoweringError::DuplicateOutput {
            name: line_output.0.to_string(),
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
    push(
        builder,
        Some(name),
        Operation::SourceMatches {
            from,
            span,
            base_line,
            pattern,
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
