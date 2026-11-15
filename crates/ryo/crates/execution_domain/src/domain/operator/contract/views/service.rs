//! The `views` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::types::{ClosureViews, PartitionCount, PartitionIndex, SourceSlice, ValidatedSource};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::source_matches::types::{BaseLine, SourceBase};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;

pub(crate) fn len<'a>(views: ClosureViews<'a>) -> PartitionCount {
    match views {
        ClosureViews::Empty => PartitionCount(0),
        ClosureViews::One(_) => PartitionCount(1),
        ClosureViews::Many(parts) => parts.len(),
    }
}

pub(crate) fn get<'a>(
    views: ClosureViews<'a>,
    at: PartitionIndex,
) -> Result<&'a ryo_derived::domain::closure_adjacency::types::ClosurePartition, OperatingError> {
    match views {
        ClosureViews::Empty => Err(OperatingError::ShapeMismatch),
        ClosureViews::One(part) => (at == PartitionIndex(0))
            .then_some(part)
            .ok_or(OperatingError::ShapeMismatch),
        ClosureViews::Many(parts) => parts.get(at),
    }
}

/// Returns the physical text only after validating the whole input span. An absent span carries no
/// occurrences; malformed boundaries fail before the row emits.
pub(super) fn source_slice<'a>(
    row: &[Value],
    span: ColumnIndex,
    source: ryo_values::domain::files::types::SourceView<'a>,
) -> Result<Option<SourceSlice<'a>>, OperatingError> {
    let Value::Span(span) = ryo_values::domain::rows::service::cell(row, span)? else {
        return Err(OperatingError::ShapeMismatch);
    };
    if span == ryo_relations::domain::relations::constants::NO_SPAN {
        return Ok(None);
    }
    let start =
        usize::try_from(span.start.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    if start > end {
        return Err(OperatingError::InvalidSourceMatchSpan);
    }
    let bytes = source
        .0
        .get(start..end)
        .ok_or(OperatingError::InvalidSourceMatchSpan)?;
    let text =
        std::str::from_utf8(bytes).map_err(|_error| OperatingError::InvalidSourceMatchSpan)?;
    Ok(Some(SourceSlice {
        base: SourceBase(start),
        text: CellText(text),
    }))
}

/// The source gate of an occurrence search: validates the span, and its base line, before any matcher is
/// acquired. An absent span suppresses the row's later checks.
///
/// # Errors
///
/// [`OperatingError::ShapeMismatch`] when the span or base-line cell is of another kind, and
/// [`OperatingError::InvalidSourceMatchSpan`] for a span out of order, outside the source or off a character
/// boundary, or a base line that is not positive; a missing column is the row's own refusal.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::service::validate;
/// ```
pub fn validate<'a>(
    row: &[Value],
    span: ColumnIndex,
    base_line: Option<ColumnIndex>,
    source: ryo_values::domain::files::types::SourceView<'a>,
) -> Result<Option<ValidatedSource<'a>>, OperatingError> {
    let Some(SourceSlice { base, text }) = source_slice(row, span, source)? else {
        return Ok(None);
    };
    let line = if let Some(column) = base_line {
        let Value::Number(ryo_relations::domain::relations::types::Integer(number)) =
            ryo_values::domain::rows::service::cell(row, column)?
        else {
            return Err(OperatingError::ShapeMismatch);
        };
        if number <= 0 {
            return Err(OperatingError::InvalidSourceMatchSpan);
        }
        BaseLine::Starts(ryo_relations::domain::relations::types::Integer(number))
    } else {
        BaseLine::Unnumbered
    };
    Ok(Some(ValidatedSource { base, text, line }))
}
