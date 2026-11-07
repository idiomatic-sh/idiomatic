//! Checked endpoint conversion; source acquisition and row publication are engine work.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{Integer, Value},
};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Return start followed by end without validating against a particular source.
pub fn bounds(row: &[Value], column: ColumnIndex) -> Result<[Value; 2], PortableOperatorFault> {
    let value = row
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(PortableOperatorFault::NoSuchColumn { index: column })?;
    let Value::Span(span) = value else {
        return Err(PortableOperatorFault::ShapeMismatch);
    };
    if span == NO_SPAN || span.start > span.end {
        return Err(PortableOperatorFault::InvalidSpanBounds);
    }
    let start =
        usize::try_from(span.start.0).map_err(|_| PortableOperatorFault::InvalidSpanBounds)?;
    let end = usize::try_from(span.end.0).map_err(|_| PortableOperatorFault::InvalidSpanBounds)?;
    let start = i64::try_from(start).map_err(|_| PortableOperatorFault::InvalidSpanBounds)?;
    let end = i64::try_from(end).map_err(|_| PortableOperatorFault::InvalidSpanBounds)?;
    Ok([Value::Number(Integer(start)), Value::Number(Integer(end))])
}
