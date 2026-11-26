//! Lazily cache the decoded grammar result once per distinct source string.
use crate::domain::format_syntax::types::RetainedFormatIndex;



use super::super::format_syntax::types::{CachedFormat, FormatParseResult, FormatParseState};





/// Reset current identities and admit small, content-only results for one following source.
pub(crate) fn clear(
    index: &mut RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>,
) {
    crate::domain::grammar_cache::service::empty(&mut index.memo);
    for (input, at) in index.by_input.drain(..) {
        if let Some(value) = index.values.get_mut(at.0) {
            let result = std::mem::replace(&mut value.result, FormatParseResult::Unavailable);
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
    index.state = FormatParseState::Fresh;
}
