use super::service::between;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_language::domain::relations::{
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
fn composed(row: &[Value], length: usize) -> Result<Value, PortableOperatorFault> {
    between(row, ColumnIndex(0), ColumnIndex(1), length)
}
#[test]
fn uses_authored_endpoints_without_union_sorting_or_input_mutation() {
    let row = [span(4, 9), span(0, 7)];
    assert_eq!(composed(&row, 9), Ok(span(4, 7)));
    assert_eq!(
        between(&row, ColumnIndex(1), ColumnIndex(0), 9),
        Ok(span(0, 9))
    );
    assert_eq!(row, [span(4, 9), span(0, 7)]);
    assert_eq!(composed(&[span(0, 0), span(0, 0)], 0), Ok(span(0, 0)));
    // Byte intervals need not begin on a UTF-8 character boundary.
    assert_eq!(
        composed(&[span(1, 2), span(2, 3)], "λx".len()),
        Ok(span(1, 3))
    );
}
#[test]
fn validates_both_present_operands_even_when_the_other_is_absent() {
    let missing = Value::Span(NO_SPAN);
    assert_eq!(composed(&[missing, missing], 0), Ok(missing));
    for located in [span(0, 4), span(3, 2)] {
        assert_eq!(
            composed(&[missing, located], 3),
            Err(PortableOperatorFault::InvalidSpan)
        );
        assert_eq!(
            composed(&[located, missing], 3),
            Err(PortableOperatorFault::InvalidSpan)
        );
    }
    assert_eq!(composed(&[missing, span(0, 3)], 3), Ok(missing));
    assert_eq!(composed(&[span(0, 3), missing], 3), Ok(missing));
    assert_eq!(
        composed(&[span(7, 8), span(0, 6)], 8),
        Err(PortableOperatorFault::InvalidSpan)
    );
}
#[test]
fn preserves_lookup_before_type_check_and_left_to_right_missing_column_errors() {
    let wrong = Value::Number(Integer(1));
    assert_eq!(
        composed(&[wrong], 0),
        Err(PortableOperatorFault::NoSuchColumn { index: 1 })
    );
    assert_eq!(
        composed(&[wrong, Value::Span(NO_SPAN)], 0),
        Err(PortableOperatorFault::ShapeMismatch)
    );
    assert_eq!(
        between(&[], ColumnIndex(8), ColumnIndex(9), 0),
        Err(PortableOperatorFault::NoSuchColumn { index: 8 })
    );
    assert_eq!(
        composed(&[span(3, 2), wrong], 3),
        Err(PortableOperatorFault::ShapeMismatch)
    );
}
#[test]
fn source_length_boundary_and_large_offsets_never_wrap() {
    assert_eq!(composed(&[span(0, 2), span(1, 3)], 3), Ok(span(0, 3)));
    assert_eq!(
        composed(&[span(0, 2), span(1, 3)], 2),
        Err(PortableOperatorFault::InvalidSpan)
    );
    let row = [span(u32::MAX, u32::MAX), span(u32::MAX, u32::MAX)];
    assert_eq!(composed(&row, 0), Err(PortableOperatorFault::InvalidSpan));
    if usize::try_from(u32::MAX).is_ok() {
        assert_eq!(composed(&row, usize::MAX), Ok(span(u32::MAX, u32::MAX)));
    } else {
        assert_eq!(
            composed(&row, usize::MAX),
            Err(PortableOperatorFault::InvalidSpan)
        );
    }
}
