//! The `records` service: free functions named by verb over what a run found of each file, with no IO and no ports.

use crate::domain::files::types::CorpusEntry;
use crate::domain::files::types::Nanoseconds;
use crate::domain::records::types::Freshness;
use crate::domain::records::types::Record;
use crate::domain::row_texts::types::{
    Cell, CellText, RenderedText, RowsTextAt, RowsTextView, TextRange,
};

/// Whether the affected scopes of a partial frontier are nonempty, sorted, name each scope once, and name every kind list sorted
/// and once each.
#[must_use]
pub fn is_well_formed(
    affected: &[crate::domain::records::types::FrontierAffected],
) -> crate::domain::records::types::FrontierShape {
    use crate::domain::records::types::{FrontierAffected, FrontierMissing, FrontierShape};
    let scope = |each: &FrontierAffected| matches!(each, FrontierAffected::Repository(_));
    let sorted_kinds = |each: &FrontierAffected| match each {
        FrontierAffected::CurrentFile(FrontierMissing::Kinds { kinds, .. })
        | FrontierAffected::Repository(FrontierMissing::Kinds { kinds, .. }) => {
            !kinds.is_empty() && kinds.windows(2).all(|pair| pair[0] < pair[1])
        }
        FrontierAffected::CurrentFile(FrontierMissing::Any)
        | FrontierAffected::Repository(FrontierMissing::Any) => true,
    };
    let well_formed = !affected.is_empty()
        && affected.iter().all(sorted_kinds)
        && affected
            .windows(2)
            .all(|pair| pair[0] < pair[1] && scope(&pair[0]) != scope(&pair[1]));
    if well_formed {
        FrontierShape::WellFormed
    } else {
        FrontierShape::Malformed
    }
}

/// The cell `lent` is, owning its text.
#[must_use]
pub fn owned(lent: Cell<CellText<'_>>) -> Cell {
    match lent {
        Cell::File(path) => Cell::File(path.0.into()),
        Cell::Text(text) => Cell::Text(text.0.into()),
        Cell::Number(number) => Cell::Number(number),
        Cell::Span(span) => Cell::Span(span),
    }
}
