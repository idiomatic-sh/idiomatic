use super::service::bounds;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{Integer, Offset, Span, Value},
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
fn span(start: u32, end: u32) -> Value {
    Value::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}
#[test]
fn preserves_endpoint_order_duplicate_rows_and_input_cells() {
    let rows = [
        [Value::Number(Integer(9)), span(4, 7)],
        [Value::Number(Integer(1)), span(0, 0)],
        [Value::Number(Integer(9)), span(4, 7)],
    ];
    let original = rows;
    let actual = rows
        .iter()
        .map(|row| bounds(row, ColumnIndex(1)))
        .collect::<Result<Vec<_>, _>>();
    assert_eq!(
        actual,
        Ok(vec![
            [Value::Number(Integer(4)), Value::Number(Integer(7))],
            [Value::Number(Integer(0)), Value::Number(Integer(0))],
            [Value::Number(Integer(4)), Value::Number(Integer(7))]
        ])
    );
    assert_eq!(rows, original);
}
#[test]
fn distinguishes_missing_columns_types_and_invalid_spans() {
    assert_eq!(
        bounds(&[], ColumnIndex(7)),
        Err(PortableOperatorFault::NoSuchColumn { index: 7 })
    );
    assert_eq!(
        bounds(&[Value::Number(Integer(2))], ColumnIndex(0)),
        Err(PortableOperatorFault::ShapeMismatch)
    );
    for value in [Value::Span(NO_SPAN), span(6, 5)] {
        assert_eq!(
            bounds(&[value], ColumnIndex(0)),
            Err(PortableOperatorFault::InvalidSpanBounds)
        );
    }
}
#[test]
fn largest_offsets_are_checked_without_integer_wrap_or_source_length_gate() {
    let result = bounds(&[span(u32::MAX, u32::MAX)], ColumnIndex(0));
    if usize::try_from(u32::MAX).is_ok() {
        assert_eq!(
            result,
            Ok([
                Value::Number(Integer(i64::from(u32::MAX))),
                Value::Number(Integer(i64::from(u32::MAX)))
            ])
        );
    } else {
        assert_eq!(result, Err(PortableOperatorFault::InvalidSpanBounds));
    }
}
