//! Match physical source bytes inside a checked span and append exact occurrence spans.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table};
use regex_automata::meta::Regex;
use ryo_language::domain::relations::constants::NO_SPAN;
use ryo_language::domain::relations::types::{CellKind, Integer, Offset, Span, Value};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;

/// Return the physical text only after validating the whole input span. An absent
/// span carries no occurrences; malformed boundaries fail before the row emits.
fn source_slice<'a>(
    row: &[Value],
    span: ColumnIndex,
    source: &'a [u8],
) -> Result<Option<(usize, &'a str)>, OperatingError> {
    let Value::Span(span) = cell(row, span)? else {
        return Err(OperatingError::ShapeMismatch);
    };
    if span == NO_SPAN {
        return Ok(None);
    }
    let start =
        usize::try_from(span.start.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    if start > end {
        return Err(OperatingError::InvalidSourceMatchSpan);
    }
    let bytes = source
        .get(start..end)
        .ok_or(OperatingError::InvalidSourceMatchSpan)?;
    let text =
        std::str::from_utf8(bytes).map_err(|_error| OperatingError::InvalidSourceMatchSpan)?;
    Ok(Some((start, text)))
}

fn ready<'a>(
    pattern: &str,
    prepared: Option<&'a crate::domain::matching::types::prepared::value::PreparedMatchers>,
    local: &'a mut Option<Regex>,
) -> Result<&'a Regex, OperatingError> {
    if let Some(found) = prepared.and_then(|prepared| prepared.regex(pattern)) {
        return found;
    }
    if local.is_none() {
        *local = Some(
            Regex::new(pattern).map_err(|error| OperatingError::InvalidRegex {
                reason: error.to_string(),
            })?,
        );
    }
    local.as_ref().ok_or(OperatingError::WrongBinding)
}

/// Advance only to the next match start. CRLF counts once after LF; a match at
/// the LF byte still belongs to the preceding physical line.
fn advance_line(
    bytes: &[u8],
    source: &[u8],
    base: usize,
    cursor: &mut usize,
    to: usize,
    line: &mut i64,
    pending_cr: &mut bool,
) -> Result<(), OperatingError> {
    for &byte in bytes
        .get(*cursor..to)
        .ok_or(OperatingError::InvalidSourceMatchSpan)?
    {
        if *pending_cr {
            *line = line
                .checked_add(1)
                .ok_or(OperatingError::InvalidSourceMatchSpan)?;
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
                .ok_or(OperatingError::InvalidSourceMatchSpan)?;
        }
    }
    *cursor = to;
    if *pending_cr && source.get(base + to) != Some(&b'\n') {
        *line = line
            .checked_add(1)
            .ok_or(OperatingError::InvalidSourceMatchSpan)?;
        *pending_cr = false;
    }
    Ok(())
}

fn occurrences(
    row: &[Value],
    span: ColumnIndex,
    base_line: Option<ColumnIndex>,
    pattern: &str,
    capture: Option<&str>,
    capture_text: bool,
    capture_line: bool,
    context: &mut OperatorContext<'_>,
    local: &mut Option<Regex>,
    joined: &mut Vec<Value>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    if (capture_text || capture_line) && capture.is_none() || capture_line && base_line.is_none() {
        return Err(OperatingError::WrongOperation);
    }
    let source = context.source.0;
    let Some((base, text)) = source_slice(row, span, source)? else {
        return Ok(());
    };
    let mut line = if let Some(column) = base_line {
        let Value::Number(Integer(number)) = cell(row, column)? else {
            return Err(OperatingError::ShapeMismatch);
        };
        if number <= 0 {
            return Err(OperatingError::InvalidSourceMatchSpan);
        }
        Some(number)
    } else {
        None
    };
    let prepared = context.prepared_matchers;
    let regex = ready(pattern, prepared, local)?;
    if let Some(name) = capture
        && regex
            .group_info()
            .to_index(regex_automata::util::primitives::PatternID::ZERO, name)
            .is_none()
    {
        return Err(OperatingError::InvalidRegex {
            reason: format!("missing named capture `{name}`"),
        });
    }
    let mut cursor = 0;
    let mut pending_cr = false;
    let mut emit_one = |start_at: usize,
                        end_at: usize,
                        captured: Option<(usize, usize)>|
     -> Result<(), OperatingError> {
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
            .ok_or(OperatingError::InvalidSourceMatchSpan)?;
        let end = base
            .checked_add(end_at)
            .ok_or(OperatingError::InvalidSourceMatchSpan)?;
        let start = u32::try_from(start).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
        let end = u32::try_from(end).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
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
                    .ok_or(OperatingError::InvalidSourceMatchSpan)?,
            )
            .map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
            let absolute_end = u32::try_from(
                base.checked_add(capture_end)
                    .ok_or(OperatingError::InvalidSourceMatchSpan)?,
            )
            .map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
            joined.push(Value::Span(Span {
                start: Offset(absolute_start),
                end: Offset(absolute_end),
            }));
            if capture_text {
                let captured_text = text
                    .get(capture_start..capture_end)
                    .ok_or(OperatingError::InvalidSourceMatchSpan)?;
                // Query execution follows provider fact reading. A fresh source-backed
                // symbol would move `sources` past existing derived provider names
                // (including comments), corrupting their text resolution.
                joined.push(Value::Symbol(query_intern_derived(
                    &mut context.symbols,
                    ByteSlice(source),
                    NameText(captured_text),
                )));
            }
            if capture_line {
                joined.push(Value::Number(Integer(
                    line.ok_or(OperatingError::InvalidSourceMatchSpan)?,
                )));
            }
        }
        emit.row(context, joined)?;
        Ok(())
    };
    if let Some(name) = capture {
        for captures in regex.captures_iter(text) {
            let Some(found) = captures.get_match() else {
                continue;
            };
            let Some(group) = captures.get_group_by_name(name) else {
                continue;
            };
            emit_one(found.start(), found.end(), Some((group.start, group.end)))?;
        }
    } else {
        for found in regex.find_iter(text) {
            emit_one(found.start(), found.end(), None)?;
        }
    }
    Ok(())
}

/// Physical occurrence search, preserving input bag/order and regex match order.
pub(crate) struct SourceMatchesItem;

impl OperatorsItem for SourceMatchesItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::SourceMatches {
            span,
            base_line,
            pattern,
            capture,
            capture_text,
            capture_line,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let input = first(inputs)?;
        reset(out, kinds_of(input));
        out.kinds.push(CellKind::Span);
        if base_line.is_some() {
            out.kinds.push(CellKind::Number);
        }
        if capture.is_some() {
            out.kinds.push(CellKind::Span);
            if *capture_text {
                out.kinds.push(CellKind::Symbol);
            }
            if *capture_line {
                out.kinds.push(CellKind::Number);
            }
        }
        let mut row = Vec::new();
        let mut joined = Vec::new();
        let mut local = None;
        for at in positions(count_of(input)) {
            let Some(each) = read_row(input, at, &mut row) else {
                continue;
            };
            occurrences(
                each,
                *span,
                *base_line,
                &pattern.0,
                capture.as_ref().map(|name| name.0.as_ref()),
                *capture_text,
                *capture_line,
                context,
                &mut local,
                &mut joined,
                &mut Collect(out),
            )?;
        }
        Ok(())
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Operation::SourceMatches {
            span,
            base_line,
            pattern,
            capture,
            capture_text,
            capture_line,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let mut joined = std::mem::take(&mut room.scratch.row);
        let mut local = None;
        let result = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                occurrences(
                    row,
                    *span,
                    *base_line,
                    &pattern.0,
                    capture.as_ref().map(|name| name.0.as_ref()),
                    *capture_text,
                    *capture_line,
                    each,
                    &mut local,
                    &mut joined,
                    emit,
                )
            }),
        );
        room.scratch.row = joined;
        result
    }
}
