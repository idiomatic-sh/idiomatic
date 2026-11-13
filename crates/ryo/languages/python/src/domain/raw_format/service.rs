//! Parse preserved logical token spellings without decoding or concatenating bodies.
use super::types::{
    RawFormatBody, RawFormatIndex, RawFormatOccurrence, RawFormatSegment, RawFormatToken,
};
use crate::domain::format_syntax::types::{FormatParseResult, FormatParseState};

/// Discard current source identities and bodies while retaining collection capacities.
pub(crate) fn clear(index: &mut RawFormatIndex) {
    crate::domain::grammar_cache::service::empty(&mut index.memo);
    for (input, at) in index.by_input.drain(..) {
        if let Some(value) = index.values.get_mut(at.0) {
            let result = std::mem::replace(value, FormatParseResult::Unavailable);
            if result != FormatParseResult::Unavailable {
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
    crate::domain::grammar_cache::service::reset(&mut index.segments);
    index.state = FormatParseState::Fresh;
}

/// Check the ordinary string-token envelope and borrow its raw body unchanged.
pub(crate) fn body(token: RawFormatToken<'_>) -> Option<RawFormatBody<'_>> {
    let token = token.0;
    let bytes = token.as_bytes();
    let prefix = usize::from(matches!(bytes.first(), Some(b'r' | b'R' | b'u' | b'U')));
    let quoted = token.get(prefix..)?;
    let quote = *quoted.as_bytes().first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let width = if quoted
        .as_bytes()
        .get(..3)
        .is_some_and(|first| first.iter().all(|byte| *byte == quote))
    {
        3
    } else {
        1
    };
    let end = quoted.len().checked_sub(width)?;
    if end < width
        || !quoted
            .as_bytes()
            .get(end..)?
            .iter()
            .all(|byte| *byte == quote)
    {
        return None;
    }
    quoted.get(width..end).map(RawFormatBody)
}

/// The token sequence belonging to one retained source expression.
pub(crate) fn segments<'a>(
    index: &'a RawFormatIndex,
    occurrence: &RawFormatOccurrence,
) -> &'a [RawFormatSegment] {
    index
        .segments
        .get(occurrence.first.0..occurrence.end.0)
        .unwrap_or_default()
}

/// A whole parser outcome, shared by equal raw bodies.
pub(crate) fn result<'a>(
    index: &'a RawFormatIndex,
    segment: &RawFormatSegment,
) -> Option<&'a FormatParseResult> {
    index.values.get(segment.cached?.0)
}

/// SyntaxCompleteness reported by either demand-driven raw reader.
pub(crate) fn state(index: &RawFormatIndex) -> FormatParseState {
    index.state
}
