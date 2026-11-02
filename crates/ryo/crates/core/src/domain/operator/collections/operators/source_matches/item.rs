//! Match physical source bytes inside a checked span and append exact occurrence spans.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::types::{Integer, Offset, Span, Value};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plans::types::{Operation, OperationKind};
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
    capture: Option<&str>,
    capture_text: bool,
    capture_line: bool,
    lease: &mut PrimitiveLease<'_>,
    joined: &mut Vec<Value>,
    emit: &mut dyn RowSink,
) -> Result<(), OperatingError> {
    if (capture_text || capture_line) && capture.is_none() {
        return Err(OperatingError::WrongOperation);
    }
    let source = lease.views.source.0;
    let checked = lease
        .views
        .validated_source
        .ok_or(OperatingError::WrongBinding)?;
    if capture_line && checked.line.is_none() {
        return Err(OperatingError::WrongOperation);
    }
    let (base, text) = (checked.base, checked.text);
    let mut line = checked.line;
    let regex = lease.views.regex.ok_or(OperatingError::WrongBinding)?;
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
                    &mut lease.derived,
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
        emit.row(lease, joined)?;
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

pub(crate) struct SourceMatchesItem;
impl OperatorsItem for SourceMatchesItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::SourceMatches,
            operation,
            inputs,
            bindings,
        )
    }
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let Operation::SourceMatches {
            capture,
            capture_text,
            capture_line,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let mut joined = std::mem::take(lease.workspace.row);
        let result = occurrences(
            row,
            capture.as_ref().map(|name| name.0.as_ref()),
            *capture_text,
            *capture_line,
            lease,
            &mut joined,
            emit,
        );
        *lease.workspace.row = joined;
        result
    }
}
