//! Validate both located operands before propagating an absent span.
use crate::domain::contract::types::{PortableOperatorFault, SourceByteLength};
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{Span, Value},
};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Return left.start through right.end, checking each present operand first.
pub fn between(
    row: &[Value],
    start: ColumnIndex,
    end: ColumnIndex,
    source_len: SourceByteLength,
) -> Result<Value, PortableOperatorFault> {
    let cell = |column: ColumnIndex| {
        row.get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
    // Both lookups precede the type pattern, including when the left type is wrong.
    let (Value::Span(left), Value::Span(right)) = (cell(start)?, cell(end)?) else {
        return Err(PortableOperatorFault::ShapeMismatch);
    };
    for span in [left, right] {
        if span != NO_SPAN {
            let span_end =
                usize::try_from(span.end.0).map_err(|_| PortableOperatorFault::InvalidSpan)?;
            if span.start > span.end || span_end > source_len.0 {
                return Err(PortableOperatorFault::InvalidSpan);
            }
        }
    }
    if left == NO_SPAN || right == NO_SPAN {
        return Ok(Value::Span(NO_SPAN));
    }
    if left.start > right.end {
        return Err(PortableOperatorFault::InvalidSpan);
    }
    Ok(Value::Span(Span {
        start: left.start,
        end: right.end,
    }))
}

/// The one lowering of a span-between: its start and end span columns, for the `text_map` kernel.
pub(crate) fn lower(start: ColumnIndex, end: ColumnIndex) -> super::types::SpanBetweenParts {
    super::types::SpanBetweenParts { start, end }
}
