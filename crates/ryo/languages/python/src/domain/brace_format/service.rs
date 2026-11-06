//! Lazily parse decoded brace-format strings once per distinct input value.
use super::super::nodes::types::Node;
use super::super::syntax::service::kind;
use super::super::syntax::types::Kind;
use super::interface::BraceFormatSyntax;
use super::types::{BraceFormatResult, BraceFormatState};
use crate::domain::brace_format::types::CachedBraceFormat;
use crate::domain::format_syntax::types::FormatOccurrence;
use crate::domain::format_syntax::types::RetainedFormatIndex;
use crate::domain::format_syntax::types::{
    DecodedInput, FormatResultIndex, FormatScratch, FormatVisibility, StringOccurrence,
};
use ryo::languages::{Position, Symbols, Tree};
use std::cell::{Ref, RefCell};

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

/// Cache one original literal occurrence by its decoded input value.
fn occurrence<P>(
    tree: &Tree<'_>,
    index: &mut RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
    at: Position,
) where
    P: BraceFormatSyntax,
{
    let mut input = std::mem::take(&mut index.memo.scratch).0;
    let decoded =
        super::super::format_parsing::service::decoded_into(tree, at, FormatScratch(&mut input));
    let (cached, unknown_input) = match decoded {
        Ok(DecodedInput::Dynamic) => (None, true),
        Err(()) => {
            index.state = BraceFormatState::Partial;
            (None, false)
        }
        Ok(DecodedInput::Static) => {
            if let Some(cached) = index.by_input.get(&input).copied() {
                (Some(cached), false)
            } else {
                if crate::domain::grammar_cache::service::reserve(&mut index.by_input)
                    == crate::domain::grammar_cache::types::Reservation::Refused
                    || index.values.try_reserve(1).is_err()
                    || index.occurrences.try_reserve(1).is_err()
                {
                    index.state = BraceFormatState::Partial;
                    return;
                }
                let cached = FormatResultIndex(index.values.len());
                let (key, result) = if let Some(taken) = crate::domain::grammar_cache::service::take(
                    &mut index.memo,
                    crate::domain::grammar_cache::types::Lookup(&input),
                ) {
                    (taken.key.0, taken.value)
                } else {
                    {
                        let result =
                            P::parse(crate::domain::format_syntax::types::FormatInput(&input));
                        (std::mem::take(&mut input), result)
                    }
                };
                if result == BraceFormatResult::Unavailable {
                    index.state = BraceFormatState::Partial;
                }
                let _prior = index.by_input.insert(key.into(), cached);
                index
                    .values
                    .push(super::types::CachedBraceFormat { result });
                (Some(cached), false)
            }
        }
    };
    crate::domain::grammar_cache::service::scratch(
        &mut index.memo,
        crate::domain::grammar_cache::types::Scratch(input),
    );
    if index.occurrences.try_reserve(1).is_err() {
        index.state = BraceFormatState::Partial;
        return;
    }
    index.occurrences.push(FormatOccurrence {
        node: at,
        cached,
        unknown_input: unknown_input.into(),
    });
}

/// Prepare once, and only when a brace-format relation is read.
/// Source order and candidate visibility come from the caller's semantic projection.
pub(crate) fn prepare<'a, P>(
    tree: &Tree<'_>,
    held: &'a RefCell<RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>>,
    symbols: &mut Symbols,
    positions: impl Iterator<Item = Position>,
    mut visibility: impl FnMut(Position, &mut Symbols) -> FormatVisibility,
) -> Ref<'a, RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>>
where
    P: BraceFormatSyntax,
{
    if crate::domain::grammar_cache::service::grammar_state::<P, _>(
        &held.borrow().memo,
        crate::domain::grammar_cache::types::DialectName(P::GRAMMAR.0),
    ) == crate::domain::grammar_cache::types::GrammarState::Stale
    {
        clear(&mut held.borrow_mut());
    }
    if held.borrow().state == BraceFormatState::Fresh {
        let mut index = held.borrow_mut();
        crate::domain::grammar_cache::service::grammar::<P, _>(
            &mut index.memo,
            crate::domain::grammar_cache::types::DialectName(P::GRAMMAR.0),
        );
        for at in positions {
            if !matches!(kind(tree, at), Kind::Node(Node::Constant | Node::JoinedStr))
                || super::super::format_parsing::service::is_string_occurrence(tree, at)
                    != StringOccurrence::String
                || visibility(at, symbols) != FormatVisibility::Visible
            {
                continue;
            }
            occurrence::<P>(tree, &mut index, at);
        }
        crate::domain::grammar_cache::service::empty(&mut index.memo);
        if index.state == BraceFormatState::Fresh {
            index.state = BraceFormatState::Complete;
        }
    }
    held.borrow()
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
