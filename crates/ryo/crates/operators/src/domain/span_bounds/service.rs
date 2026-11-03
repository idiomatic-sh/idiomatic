//! Checked endpoint conversion; source acquisition and row publication are engine work.
use crate::domain::contract::types::SpanFault;
use ryo_language::domain::relations::{
    constants::NO_SPAN,
    types::{Integer, Value},
};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Return start followed by end without validating against a particular source.
pub fn bounds(row: &[Value], column: ColumnIndex) -> Result<[Value; 2], SpanFault> {
    let value = row
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(SpanFault::NoSuchColumn { index: column.0 })?;
    let Value::Span(span) = value else {
        return Err(SpanFault::ShapeMismatch);
    };
    if span == NO_SPAN || span.start > span.end {
        return Err(SpanFault::InvalidSpanBounds);
    }
    let start = usize::try_from(span.start.0).map_err(|_| SpanFault::InvalidSpanBounds)?;
    let end = usize::try_from(span.end.0).map_err(|_| SpanFault::InvalidSpanBounds)?;
    let start = i64::try_from(start).map_err(|_| SpanFault::InvalidSpanBounds)?;
    let end = i64::try_from(end).map_err(|_| SpanFault::InvalidSpanBounds)?;
    Ok([Value::Number(Integer(start)), Value::Number(Integer(end))])
}
