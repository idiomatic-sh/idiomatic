//! Checked physical-source gate, shared by the engine and occurrence evaluator.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service::cell;
use ryo_language::domain::relations::constants::NO_SPAN;
use ryo_language::domain::relations::types::{Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ValidatedSource<'a> {
    pub base: usize,
    pub text: &'a str,
    pub line: Option<i64>,
}

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

/// Validate before any regex is acquired. An absent span suppresses subsequent row checks.
pub(crate) fn validate<'a>(
    row: &[Value],
    span: ColumnIndex,
    base_line: Option<ColumnIndex>,
    source: &'a [u8],
) -> Result<Option<ValidatedSource<'a>>, OperatingError> {
    let Some((base, text)) = source_slice(row, span, source)? else {
        return Ok(None);
    };
    let line = if let Some(column) = base_line {
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
    Ok(Some(ValidatedSource { base, text, line }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ryo_language::domain::relations::types::{Offset, Span};
    fn span(start: u32, end: u32) -> Value {
        Value::Span(Span {
            start: Offset(start),
            end: Offset(end),
        })
    }
    #[test]
    fn checked_gate_retains_physical_offsets_and_positive_line() {
        let source = "xé\r\ny".as_bytes();
        let row = [span(1, 5), Value::Number(Integer(7))];
        let value = validate(&row, ColumnIndex(0), Some(ColumnIndex(1)), source)
            .unwrap()
            .unwrap();
        assert_eq!((value.base, value.text, value.line), (1, "é\r\n", Some(7)));
    }
    #[test]
    fn absent_span_skips_line_check_but_invalid_boundaries_fail() {
        let source = "é".as_bytes();
        assert!(
            validate(
                &[Value::Span(NO_SPAN)],
                ColumnIndex(0),
                Some(ColumnIndex(99)),
                source
            )
            .unwrap()
            .is_none()
        );
        for value in [span(1, 2), span(2, 1), span(0, 3)] {
            assert!(matches!(
                validate(&[value], ColumnIndex(0), None, source),
                Err(OperatingError::InvalidSourceMatchSpan)
            ));
        }
        assert!(matches!(
            validate(
                &[span(0, 2), Value::Number(Integer(0))],
                ColumnIndex(0),
                Some(ColumnIndex(1)),
                source
            ),
            Err(OperatingError::InvalidSourceMatchSpan)
        ));
    }
}
