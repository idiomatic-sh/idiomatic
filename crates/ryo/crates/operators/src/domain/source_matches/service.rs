//! Physical occurrence span, capture and line semantics over one borrowed checked interval.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::{SourceMatchesInput, TextRowDriver};
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase, use_case::MatchingUseCaseImpl,
};
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value};
use std::ops::ControlFlow;
fn advance_line(
    bytes: &[u8],
    source: &[u8],
    base: usize,
    cursor: &mut usize,
    to: usize,
    line: &mut i64,
    pending_cr: &mut bool,
) -> Result<(), PortableOperatorFault> {
    for &byte in bytes
        .get(*cursor..to)
        .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?
    {
        if *pending_cr {
            *line = line
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
            *pending_cr = false;
            if byte == b'\n' {
                continue;
            }
        }
        if byte == b'\r' {
            *pending_cr = true;
        } else if byte == b'\n' {
            *line = line
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        }
    }
    *cursor = to;
    if *pending_cr && source.get(base + to) != Some(&b'\n') {
        *line = line
            .checked_add(1)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        *pending_cr = false;
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
        line: mut line,
        regex,
        capture,
        capture_text,
        capture_line,
    } = input;
    if (capture_text || capture_line) && capture.is_none() {
        return Err(PortableOperatorFault::WrongOperation);
    }
    if capture_line && line.is_none() {
        return Err(PortableOperatorFault::WrongOperation);
    }
    let mut cursor = 0;
    let mut pending_cr = false;
    let mut emit_one = |start_at: usize,
                        end_at: usize,
                        captured: Option<(usize, usize)>|
     -> Result<(), PortableOperatorFault> {
        let mut whole_line = None;
        if let Some(current) = line.as_mut() {
            advance_line(
                text.as_bytes(),
                source,
                base,
                &mut cursor,
                start_at,
                current,
                &mut pending_cr,
            )?;
            whole_line = Some(*current);
            if capture_line && let Some((capture_start, _)) = captured {
                advance_line(
                    text.as_bytes(),
                    source,
                    base,
                    &mut cursor,
                    capture_start,
                    current,
                    &mut pending_cr,
                )?;
            }
        }
        let start = base
            .checked_add(start_at)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        let end = base
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
            joined.push(Value::Number(Integer(current)));
        }
        if let Some((capture_start, capture_end)) = captured {
            let absolute_start = u32::try_from(
                base.checked_add(capture_start)
                    .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
            )
            .map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
            let absolute_end = u32::try_from(
                base.checked_add(capture_end)
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
                joined.push(Value::Number(Integer(
                    line.ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
                )));
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
            .visit_regex_captures(regex, text, name, &mut |found, group| match emit_one(
                found.start,
                found.end,
                Some((group.start, group.end)),
            ) {
                Ok(()) => ControlFlow::Continue(()),
                Err(error) => {
                    failure = Some(error);
                    ControlFlow::Break(())
                }
            })
            .map_err(|_| PortableOperatorFault::MissingCapture)?;
    } else {
        let _flow =
            MatchingUseCaseImpl::new().visit_regex_matches(
                regex,
                text,
                &mut |found| match emit_one(found.start, found.end, None) {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(error) => {
                        failure = Some(error);
                        ControlFlow::Break(())
                    }
                },
            );
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}
