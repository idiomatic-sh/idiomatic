//! The `format_parsing` operation over its parser port: it drives the injected parser over a tree's occurrences and retains what it answers.
use crate::domain::format_parsing::service::{clear, decoded_into, is_string_occurrence};
use crate::domain::format_syntax::types::FormatResultIndex;
use crate::domain::format_syntax::types::RetainedFormatIndex;
use crate::domain::format_syntax::types::{
    CachedFormat, DecodedInput, FormatDialect, FormatOccurrence, FormatParseResult,
    FormatParseState, FormatScratch, FormatVisibility, StringOccurrence,
};
use crate::domain::syntax::service::standing;
use crate::domain::syntax::types::Standing;
use crate::outbound::format_syntax::interface::FormatSyntaxPort;
use ryo::languages::{self, Position, Symbols, Tree};
use std::cell::{Ref, RefCell};

/// Add one occurrence, sharing a prior parse when its decoded value has already appeared.
pub(crate) fn occurrence<P>(
    tree: &Tree<'_>,
    index: &mut RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>,
    at: Position,
) where
    P: FormatSyntaxPort,
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
    P: FormatSyntaxPort,
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
        if languages::has_incomplete_syntax(tree.elements)
            == languages::SyntaxCompleteness::Incomplete
            || languages::has_incomplete_lexical_stream(tree.elements)
                == languages::SyntaxCompleteness::Incomplete
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
