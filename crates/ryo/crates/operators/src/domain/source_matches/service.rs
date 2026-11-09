//! Physical occurrence span, capture and line semantics over one borrowed checked interval.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::{
    BaseLine, CaptureRequest, CarriageReturn, LineBytes, LineCursor, OccurrenceByte,
    OccurrenceSpan, SourceBase, SourceMatchesInput, TextRowDriver,
};
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value};
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::row_texts::types::CellText;
use std::ops::ControlFlow;
fn advance_line(
    bytes: LineBytes<'_>,
    source: LineBytes<'_>,
    base: SourceBase,
    cursor: &mut LineCursor,
    to: LineCursor,
    line: &mut Integer,
    pending_cr: &mut CarriageReturn,
) -> Result<(), PortableOperatorFault> {
    for &byte in bytes
        .0
        .get(cursor.0..to.0)
        .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?
    {
        if *pending_cr == CarriageReturn::Pending {
            line.0 = line
                .0
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
            *pending_cr = CarriageReturn::Clear;
            if byte == b'\n' {
                continue;
            }
        }
        if byte == b'\r' {
            *pending_cr = CarriageReturn::Pending;
        } else if byte == b'\n' {
            line.0 = line
                .0
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        }
    }
    *cursor = to;
    if *pending_cr == CarriageReturn::Pending && source.0.get(base.0 + to.0) != Some(&b'\n') {
        line.0 = line
            .0
            .checked_add(1)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        *pending_cr = CarriageReturn::Clear;
    }
    Ok(())
}

/// Visits the occurrences of the input's pattern: each whole span with the span of the captured group, when one was asked for.
pub(crate) type VisitOccurrences<'visit> =
    &'visit dyn Fn(
        &mut dyn FnMut(std::ops::Range<usize>, Option<std::ops::Range<usize>>) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, PortableOperatorFault>;

pub(crate) fn occurrences(
    input: SourceMatchesInput<'_>,
    driver: &mut dyn TextRowDriver,
    joined: &mut Vec<Value>,
    visit: VisitOccurrences<'_>,
) -> Result<(), PortableOperatorFault> {
    let SourceMatchesInput {
        row,
        source,
        base,
        text,
        line,
        regex: _,
        capture,
        capture_text,
        capture_line,
    } = input;
    let (source, text) = (source.0, text.0);
    let capture_text = capture_text == TextOutput::Appended;
    let capture_line = capture_line == LineOutput::Appended;
    let mut line = match line {
        BaseLine::Unnumbered => None,
        BaseLine::Starts(first) => Some(first),
    };
    if (capture_text || capture_line) && capture == CaptureRequest::Whole {
        return Err(PortableOperatorFault::WrongOperation);
    }
    if capture_line && line.is_none() {
        return Err(PortableOperatorFault::WrongOperation);
    }
    let mut cursor = LineCursor(0);
    let mut pending_cr = CarriageReturn::Clear;
    let mut emit_one = |whole: OccurrenceSpan,
                        captured: Option<OccurrenceSpan>|
     -> Result<(), PortableOperatorFault> {
        let (start_at, end_at) = (whole.start.0, whole.end.0);
        let mut whole_line = None;
        if let Some(current) = line.as_mut() {
            advance_line(
                LineBytes(text.as_bytes()),
                LineBytes(source),
                base,
                &mut cursor,
                LineCursor(start_at),
                current,
                &mut pending_cr,
            )?;
            whole_line = Some(*current);
            if capture_line && let Some(group) = &captured {
                advance_line(
                    LineBytes(text.as_bytes()),
                    LineBytes(source),
                    base,
                    &mut cursor,
                    LineCursor(group.start.0),
                    current,
                    &mut pending_cr,
                )?;
            }
        }
        let start = base
            .0
            .checked_add(start_at)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        let end = base
            .0
            .checked_add(end_at)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        let start =
            u32::try_from(start).map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
        let end = u32::try_from(end).map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
        joined.clear();
        joined.extend_from_slice(row);
        joined.push(Value::Span(Span {
            start: Offset(start),
            end: Offset(end),
        }));
        if let Some(current) = whole_line {
            joined.push(Value::Number(current));
        }
        if let Some(group) = captured {
            let (capture_start, capture_end) = (group.start.0, group.end.0);
            let absolute_start = u32::try_from(
                base.0
                    .checked_add(capture_start)
                    .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
            )
            .map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
            let absolute_end = u32::try_from(
                base.0
                    .checked_add(capture_end)
                    .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
            )
            .map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
            joined.push(Value::Span(Span {
                start: Offset(absolute_start),
                end: Offset(absolute_end),
            }));
            if capture_text {
                let captured_text = text
                    .get(capture_start..capture_end)
                    .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
                // Query execution follows provider fact reading. A fresh source-backed
                // symbol would move `sources` past existing derived provider names
                // (including comments), corrupting their text resolution.
                joined.push(driver.intern(CellText(captured_text)));
            }
            if capture_line {
                joined.push(Value::Number(
                    line.ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
                ));
            }
        }
        driver
            .emit(joined)
            .map_err(|_| PortableOperatorFault::Interrupted)?;
        Ok(())
    };
    let mut failure = None;
    let _flow = visit(&mut |whole, group| match emit_one(
        OccurrenceSpan {
            start: OccurrenceByte(whole.start),
            end: OccurrenceByte(whole.end),
        },
        group.map(|group| OccurrenceSpan {
            start: OccurrenceByte(group.start),
            end: OccurrenceByte(group.end),
        }),
    ) {
        Ok(()) => ControlFlow::Continue(()),
        Err(error) => {
            failure = Some(error);
            ControlFlow::Break(())
        }
    })?;
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}

/// Emits the occurrences of the acquired matcher inside the span text the engine validated before it
/// acquired the matcher.
pub(crate) fn evaluate(
    row: &[Value],
    shape: super::types::SearchShape<'_>,
    views: super::types::SearchViews<'_>,
    driver: &mut dyn crate::domain::source_matches::types::TextRowsEvaluation,
    joined: &mut Vec<Value>,
    matching: &dyn crate::domain::contract::types::Occurrences,
) -> Result<(), PortableOperatorFault> {
    let super::types::SearchViews {
        source,
        checked,
        regex,
    } = views;
    let text = checked.text.0;
    let capture = shape.capture;
    occurrences(
        SourceMatchesInput {
            row,
            source,
            base: checked.base,
            text: checked.text,
            line: checked.line,
            regex,
            capture,
            capture_text: shape.capture_text,
            capture_line: shape.capture_line,
        },
        driver.text_rows(),
        joined,
        &|each| match capture {
            CaptureRequest::Group(name) => matching.captures(
                regex,
                ryo_matching::inbound::use_cases::matching::types::SearchText(text),
                ryo_matching::inbound::use_cases::matching::types::GroupName(name.0),
                &mut |found, group| each(found.0, Some(group.0)),
            ),
            CaptureRequest::Whole => Ok(matching.matches(
                regex,
                ryo_matching::inbound::use_cases::matching::types::SearchText(text),
                &mut |found| each(found.0, None),
            )),
        },
    )
}
