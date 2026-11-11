//! Lazily parse decoded brace-format strings once per distinct input value.
use super::types::{BraceFormatResult, BraceFormatState};
use crate::domain::brace_format::types::CachedBraceFormat;
use crate::domain::format_syntax::types::FormatOccurrence;
use crate::domain::format_syntax::types::RetainedFormatIndex;

/// Reset source identities; retain only byte-budgeted content-only grammar results.
pub(crate) fn clear(
    index: &mut RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
) {
    crate::domain::grammar_cache::service::empty(&mut index.memo);
    for (input, at) in index.by_input.drain(..) {
        if let Some(value) = index.values.get_mut(at.0) {
            let result = std::mem::replace(&mut value.result, BraceFormatResult::Unavailable);
            if result != BraceFormatResult::Unavailable {
                crate::domain::grammar_cache::service::admit(
                    &mut index.memo,
                    crate::domain::grammar_cache::types::MemoKey(input.0),
                    result,
                );
            }
        }
    }
    crate::domain::grammar_cache::service::reset_map(&mut index.by_input);
    crate::domain::grammar_cache::service::reset(&mut index.values);
    crate::domain::grammar_cache::service::reset(&mut index.occurrences);
    index.state = BraceFormatState::Fresh;
}

/// Occurrences in source order.
#[must_use]
pub(crate) fn occurrences(
    index: &RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
) -> &[FormatOccurrence] {
    &index.occurrences
}

/// State of the retained index.
#[must_use]
pub(crate) fn state(
    index: &RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
) -> BraceFormatState {
    index.state
}

/// Result associated with an occurrence.
#[must_use]
pub(crate) fn result<'a>(
    index: &'a RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
    occurrence: &FormatOccurrence,
) -> Option<&'a BraceFormatResult> {
    index
        .values
        .get(occurrence.cached?.0)
        .map(|cached| &cached.result)
}
