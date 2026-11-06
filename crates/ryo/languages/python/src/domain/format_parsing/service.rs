//! Lazily cache the decoded grammar result once per distinct source string.
use crate::domain::format_syntax::types::FormatResultIndex;
use crate::domain::format_syntax::types::RetainedFormatIndex;
use std::cell::{Ref, RefCell};

use ryo::languages::{self, Position, Symbols, Tree};

use super::super::format_syntax::interface::FormatSyntax;
use super::super::format_syntax::types::{
    CachedFormat, DecodedInput, FormatDialect, FormatOccurrence, FormatParseResult,
    FormatParseState, FormatScratch, FormatVisibility, StringOccurrence,
};
use super::super::nodes::types::Node;
use super::super::syntax::service::{below, interpolation, kind, literal, standing};
use super::super::syntax::types::{Interpolation, Kind, Literal, Standing};
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

/// Add one occurrence, sharing a prior parse when its decoded value has already appeared.
fn occurrence<P>(
    tree: &Tree<'_>,
    index: &mut RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>,
    at: Position,
) where
    P: FormatSyntax,
{
    let mut decoded = std::mem::take(&mut index.memo.scratch).0;
    match decoded_into(tree, at, FormatScratch(&mut decoded)) {
        Ok(DecodedInput::Static) => {}
        Ok(DecodedInput::Dynamic) => {
            crate::domain::grammar_cache::service::scratch(
                &mut index.memo,
                crate::domain::grammar_cache::types::Scratch(decoded),
            );
            if index.occurrences.try_reserve(1).is_err() {
                index.state = FormatParseState::Partial;
                return;
            }
            index.occurrences.push(FormatOccurrence {
                node: at,
                cached: None,
                unknown_input: true.into(),
            });
            return;
        }
        Err(()) => {
            crate::domain::grammar_cache::service::scratch(
                &mut index.memo,
                crate::domain::grammar_cache::types::Scratch(decoded),
            );
            index.state = FormatParseState::Partial;
            if index.occurrences.try_reserve(1).is_ok() {
                index.occurrences.push(FormatOccurrence {
                    node: at,
                    cached: None,
                    unknown_input: false.into(),
                });
            }
            return;
        }
    }

    if let Some(cached) = index.by_input.get(decoded.as_str()).copied() {
        if index.occurrences.try_reserve(1).is_err() {
            index.state = FormatParseState::Partial;
            return;
        }
        crate::domain::grammar_cache::service::scratch(
            &mut index.memo,
            crate::domain::grammar_cache::types::Scratch(decoded),
        );
        index.occurrences.push(FormatOccurrence {
            node: at,
            cached: Some(cached),
            unknown_input: false.into(),
        });
        return;
    }

    if index.values.try_reserve(1).is_err()
        || index.occurrences.try_reserve(1).is_err()
        || crate::domain::grammar_cache::service::reserve(&mut index.by_input)
            == crate::domain::grammar_cache::types::Reservation::Refused
    {
        index.state = FormatParseState::Partial;
        return;
    }
    let (decoded, parsed) = if let Some(taken) = crate::domain::grammar_cache::service::take(
        &mut index.memo,
        crate::domain::grammar_cache::types::Lookup(&decoded),
    ) {
        crate::domain::grammar_cache::service::scratch(
            &mut index.memo,
            crate::domain::grammar_cache::types::Scratch(decoded),
        );
        (taken.key.0, taken.value)
    } else {
        let parsed = P::parse(
            crate::domain::format_syntax::types::FormatInput(&decoded),
            FormatDialect::PercentString,
        );
        (decoded, parsed)
    };
    if parsed == FormatParseResult::Unavailable {
        index.state = FormatParseState::Partial;
    }
    let cached = FormatResultIndex(index.values.len());
    index.values.push(CachedFormat { result: parsed });
    let _prior = index.by_input.insert(decoded.into(), cached);
    index.occurrences.push(FormatOccurrence {
        node: at,
        cached: Some(cached),
        unknown_input: false.into(),
    });
}

/// Build the string occurrence index and parse each distinct decoded input at most once.
/// The caller supplies semantic source order and visibility; only candidate strings ask
/// for visibility, and an already prepared index consumes neither operation.
pub(crate) fn prepare<'a, P>(
    tree: &Tree<'_>,
    held: &'a RefCell<RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>>,
    symbols: &mut Symbols,
    positions: impl Iterator<Item = Position>,
    mut visibility: impl FnMut(Position, &mut Symbols) -> FormatVisibility,
) -> Ref<'a, RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>>
where
    P: FormatSyntax,
{
    if crate::domain::grammar_cache::service::grammar_state::<P, _>(
        &held.borrow().memo,
        crate::domain::grammar_cache::types::DialectName(P::DECODED_GRAMMAR.0),
    ) == crate::domain::grammar_cache::types::GrammarState::Stale
    {
        clear(&mut held.borrow_mut());
    }
    if held.borrow().state == FormatParseState::Fresh {
        let mut index = held.borrow_mut();
        crate::domain::grammar_cache::service::grammar::<P, _>(
            &mut index.memo,
            crate::domain::grammar_cache::types::DialectName(P::DECODED_GRAMMAR.0),
        );
        if languages::has_incomplete_syntax(tree.elements) == languages::SyntaxCompleteness::Incomplete
            || languages::has_incomplete_lexical_stream(tree.elements) == languages::SyntaxCompleteness::Incomplete
        {
            index.state = FormatParseState::Partial;
        } else {
            index.state = FormatParseState::Complete;
        }
        for at in positions {
            if is_string_occurrence(tree, at) != StringOccurrence::String
                || standing(tree, at) != Standing::Own
                || visibility(at, symbols) != FormatVisibility::Visible
            {
                continue;
            }
            occurrence::<P>(tree, &mut index, at);
        }
        crate::domain::grammar_cache::service::empty(&mut index.memo);
    }
    held.borrow()
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
