//! The `views` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::types::{ClosureViews, PartitionCount, PartitionIndex, ValidatedSource};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

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
    use ryo_operators::domain::source_matches::types::{BaseLineColumn, SearchedSpan};
    let searched = ryo_operators::domain::source_matches::service::checked_source(
        row,
        span,
        base_line.map_or(BaseLineColumn::Unnumbered, BaseLineColumn::At),
        source,
    )
    .map_err(crate::domain::operating::service::portable_fault)?;
    Ok(match searched {
        SearchedSpan::Absent => None,
        SearchedSpan::Checked(checked) => Some(ValidatedSource {
            base: checked.base,
            text: checked.text,
            line: checked.line,
        }),
    })
}
