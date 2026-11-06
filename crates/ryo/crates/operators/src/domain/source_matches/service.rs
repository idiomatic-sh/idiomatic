//! Physical occurrence span, capture and line semantics over one borrowed checked interval.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::{
    CarriageReturn, LineBytes, LineCursor, SourceBase, SourceMatchesInput, TextRowDriver,
};
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase,
    types::{GroupName, SearchText, TextSpan},
    use_case::MatchingUseCaseImpl,
};
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value};
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

pub(crate) fn occurrences(
    input: SourceMatchesInput<'_>,
    driver: &mut dyn TextRowDriver,
    joined: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let SourceMatchesInput {
        row,
        source,
        base,
        text,
        line,
        regex,
        capture,
        capture_text,
        capture_line,
    } = input;
    let base = SourceBase(base);
    let mut line = line.map(Integer);
    if (capture_text || capture_line) && capture.is_none() {
        return Err(PortableOperatorFault::WrongOperation);
    }
    if capture_line && line.is_none() {
        return Err(PortableOperatorFault::WrongOperation);
    }
    let mut cursor = LineCursor(0);
    let mut pending_cr = CarriageReturn::Clear;
    let mut emit_one = |whole: TextSpan,
                        captured: Option<TextSpan>|
     -> Result<(), PortableOperatorFault> {
        let (start_at, end_at) = (whole.0.start, whole.0.end);
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
                    LineCursor(group.0.start),
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
            let (capture_start, capture_end) = (group.0.start, group.0.end);
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
                joined.push(driver.intern(captured_text));
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
    if let Some(name) = capture {
        let _flow = MatchingUseCaseImpl::new()
            .visit_regex_captures(
                regex,
                SearchText(text),
                GroupName(name),
                &mut |found, group| match emit_one(found, Some(group)) {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(error) => {
                        failure = Some(error);
                        ControlFlow::Break(())
                    }
                },
            )
            .map_err(|_| PortableOperatorFault::MissingCapture)?;
    } else {
        let _flow =
            MatchingUseCaseImpl::new().visit_regex_matches(regex, SearchText(text), &mut |found| {
                match emit_one(found, None) {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(error) => {
                        failure = Some(error);
                        ControlFlow::Break(())
                    }
                }
            });
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}
