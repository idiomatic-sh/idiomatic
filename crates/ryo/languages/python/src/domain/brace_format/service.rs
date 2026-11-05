//! Lazily parse decoded brace-format strings once per distinct input value.
use super::super::model::types::Declarations;
use super::super::nodes::types::Node;
use super::super::semantic_syntax::types::Visibility;
use super::super::syntax::service::kind;
use super::super::syntax::types::Kind;
use super::interface::BraceFormatSyntax;
use super::types::{BraceFormatIndex, BraceFormatOccurrence, BraceFormatResult, BraceFormatState};
use crate::domain::format_syntax::types::{
    DecodedInput, FormatOwnedBytes, FormatResultIndex, FormatScratch, StringOccurrence,
};
use ryo::languages::{Position, Symbols, Tree};
use std::cell::{Ref, RefCell};

/// Reset source identities; retain only byte-budgeted content-only grammar results.
pub(crate) fn clear(index: &mut BraceFormatIndex) {
    crate::domain::grammar_cache::service::empty(&mut index.memo);
    for (input, at) in index.by_input.drain(..) {
        if let Some(value) = index.values.get_mut(at.0) {
            let result = std::mem::replace(&mut value.result, BraceFormatResult::Unavailable);
            let bytes = owned_bytes(&result);
            if result != BraceFormatResult::Unavailable {
                crate::domain::grammar_cache::service::admit(
                    &mut index.memo,
                    input.0,
                    result,
                    bytes,
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
fn occurrence<P>(tree: &Tree<'_>, index: &mut BraceFormatIndex, at: Position)
where
    P: BraceFormatSyntax,
{
    let mut input = std::mem::take(&mut index.memo.scratch);
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
                if !crate::domain::grammar_cache::service::reserve(&mut index.by_input)
                    || index.values.try_reserve(1).is_err()
                    || index.occurrences.try_reserve(1).is_err()
                {
                    index.state = BraceFormatState::Partial;
                    return;
                }
                let cached = FormatResultIndex(index.values.len());
                let (key, result) = if let Some((key, result)) =
                    crate::domain::grammar_cache::service::take(&mut index.memo, &input)
                {
                    (key, result)
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
    crate::domain::grammar_cache::service::scratch(&mut index.memo, input);
    if index.occurrences.try_reserve(1).is_err() {
        index.state = BraceFormatState::Partial;
        return;
    }
    index.occurrences.push(BraceFormatOccurrence {
        node: at,
        cached,
        unknown_input: unknown_input.into(),
    });
}

/// Prepare once, and only when a brace-format relation is read.
pub(crate) fn prepare<'a, P>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Ref<'a, BraceFormatIndex>
where
    P: BraceFormatSyntax,
{
    if !crate::domain::grammar_cache::service::is_grammar::<P, _>(
        &model.brace_format.borrow().memo,
        P::GRAMMAR.0,
    ) {
        clear(&mut model.brace_format.borrow_mut());
    }
    let held: &RefCell<BraceFormatIndex> = &model.brace_format;
    if held.borrow().state == BraceFormatState::Fresh {
        let mut index = held.borrow_mut();
        crate::domain::grammar_cache::service::grammar::<P, _>(&mut index.memo, P::GRAMMAR.0);
        for at in super::super::semantic_syntax::service::positions(tree) {
            if !matches!(kind(tree, at), Kind::Node(Node::Constant | Node::JoinedStr))
                || super::super::format_parsing::service::is_string_occurrence(tree, at)
                    != StringOccurrence::String
                || super::super::semantic_syntax::service::visibility(tree, model, at, symbols)
                    != Visibility::Visible
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
pub(crate) fn occurrences(index: &BraceFormatIndex) -> &[BraceFormatOccurrence] {
    &index.occurrences
}

/// State of the retained index.
#[must_use]
pub(crate) fn state(index: &BraceFormatIndex) -> BraceFormatState {
    index.state
}

/// Result associated with an occurrence.
#[must_use]
pub(crate) fn result<'a>(
    index: &'a BraceFormatIndex,
    occurrence: &BraceFormatOccurrence,
) -> Option<&'a BraceFormatResult> {
    index
        .values
        .get(occurrence.cached?.0)
        .map(|cached| &cached.result)
}

/// Owned result buffers, independent of the file occurrence that uses the result.
#[must_use]
pub(crate) fn owned_bytes(result: &BraceFormatResult) -> FormatOwnedBytes {
    FormatOwnedBytes(match result {
        BraceFormatResult::Parsed(parsed) => parsed
            .fields
            .capacity()
            .saturating_mul(size_of::<super::types::BraceFormatField>())
            .saturating_add(
                parsed
                    .fields
                    .iter()
                    .map(|field| {
                        field
                            .format_spec
                            .as_ref()
                            .map_or(0, |spec| spec.0.capacity())
                            .saturating_add(field.selector_text.0.capacity())
                            .saturating_add(match &field.selector {
                                super::types::BraceSelector::Automatic { .. } => 0,
                                super::types::BraceSelector::Index { text, .. }
                                | super::types::BraceSelector::Keyword(text) => text.0.capacity(),
                            })
                    })
                    .sum::<usize>(),
            ),
        BraceFormatResult::Malformed(_) | BraceFormatResult::Unavailable => 0,
    })
}
