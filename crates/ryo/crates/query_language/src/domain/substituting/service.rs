//! Type and pattern validation for the replacement projection.

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::lowering::types::PlannedColumn;
use crate::domain::lowering::types::SearchMatch;
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
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_syntax::types::Call;
use ryo_values::domain::query_tokens::types::QuerySpelling;
use ryo_values::domain::query_tokens::types::StepName;

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
    // Parsed, not compiled: the one compile of the pattern is the executor's prepared slot, on first use.
    let parsed = regex_automata::util::syntax::parse(&pattern.0).map_err(|error| {
        LoweringError::InvalidRegex {
            reason: error.to_string(),
            span: call.span,
        }
    })?;
    if let Some(name) = &capture_name
        && names_capture(&parsed, name) == SearchMatch::Differs
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
                ryo_values::domain::query_operators::types::TextOutput::Appended
            } else {
                ryo_values::domain::query_operators::types::TextOutput::Omitted
            },
            capture_line: if capture_line {
                ryo_values::domain::query_operators::types::LineOutput::Appended
            } else {
                ryo_values::domain::query_operators::types::LineOutput::Omitted
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
    // Parsed, not compiled: the one compile of the pattern is the executor's prepared slot, on first use.
    let _parsed = regex_automata::util::syntax::parse(&pattern.0).map_err(|error| {
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

/// Whether the parsed pattern has a capture group called `name`: the groups the compiled matcher numbers are the ones its syntax
/// tree holds.
fn names_capture(
    pattern: &regex_syntax::hir::Hir,
    name: &ryo_values::domain::descriptions::types::ColumnName,
) -> SearchMatch {
    let mut pending = vec![pattern];
    while let Some(hir) = pending.pop() {
        match hir.kind() {
            regex_syntax::hir::HirKind::Capture(capture) => {
                if capture.name.as_deref() == Some(&*name.0) {
                    return SearchMatch::Matches;
                }
                pending.push(&capture.sub);
            }
            regex_syntax::hir::HirKind::Repetition(repetition) => pending.push(&repetition.sub),
            regex_syntax::hir::HirKind::Concat(parts)
            | regex_syntax::hir::HirKind::Alternation(parts) => pending.extend(parts.iter()),
            regex_syntax::hir::HirKind::Empty
            | regex_syntax::hir::HirKind::Literal(_)
            | regex_syntax::hir::HirKind::Class(_)
            | regex_syntax::hir::HirKind::Look(_) => {}
        }
    }
    SearchMatch::Differs
}
