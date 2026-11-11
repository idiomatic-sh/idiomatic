//! Lazily cache the decoded grammar result once per distinct source string.
use crate::domain::format_syntax::types::RetainedFormatIndex;

use ryo::languages::{self, Position, Tree};

use super::super::format_syntax::types::{
    CachedFormat, DecodedInput, FormatOccurrence, FormatParseResult, FormatParseState,
    FormatScratch, StringOccurrence,
};
use super::super::nodes::types::Node;
use super::super::syntax::service::{below, interpolation, kind, literal};
use super::super::syntax::types::{Interpolation, Kind, Literal};
use super::super::tokens::types::Token;

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

/// Whether this is one original visible string-literal node.
pub(crate) fn is_string_occurrence(tree: &Tree<'_>, at: Position) -> StringOccurrence {
    if matches!(kind(tree, at), Kind::Node(Node::JoinedStr))
        || (kind(tree, at) == Kind::Node(Node::Constant) && literal(tree, at) == Literal::String)
    {
        StringOccurrence::String
    } else {
        StringOccurrence::Other
    }
}

/// Concatenate parser-decoded string pieces. Dynamic interpolations are known unknown inputs;
/// missing decoded parser payload is unavailable and must make the relation partial.
pub(crate) fn decoded_into(
    tree: &Tree<'_>,
    at: Position,
    text: FormatScratch<'_>,
) -> Result<DecodedInput, ()> {
    let text = text.0;
    text.clear();
    if interpolation(tree, at) == Interpolation::Interpolates {
        return Ok(DecodedInput::Dynamic);
    }
    for token in below(tree, at).filter(|each| {
        matches!(
            kind(tree, *each),
            Kind::Token(Token::String | Token::FstringMiddle | Token::TstringMiddle)
        )
    }) {
        let Some(piece) = languages::decoded_at(tree.elements, token) else {
            return Err(());
        };
        if text.try_reserve(piece.0.len()).is_err() {
            return Err(());
        }
        text.push_str(piece.0);
    }
    Ok(DecodedInput::Static)
}

/// The current completeness state of the retained parse.
#[must_use]
pub(crate) fn state(
    index: &RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>,
) -> FormatParseState {
    index.state
}

/// The original visible string occurrences and their cached parsed result indexes.
#[must_use]
pub(crate) fn occurrences(
    index: &RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>,
) -> &[FormatOccurrence] {
    &index.occurrences
}

/// The cached result for one occurrence, when the occurrence was static.
#[must_use]
pub(crate) fn result<'a>(
    index: &'a RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>,
    occurrence: &FormatOccurrence,
) -> Option<&'a FormatParseResult> {
    index
        .values
        .get(occurrence.cached?.0)
        .map(|cached| &cached.result)
}
