//! Lazily cache the decoded grammar result once per distinct source string.
use crate::domain::format_syntax::types::FormatResultIndex;
use std::cell::{Ref, RefCell};

use ryo::languages::{self, Position, Symbols, Tree};

use super::super::format_syntax::interface::FormatSyntax;
use super::super::format_syntax::types::{
    CachedFormat, DecodedInput, FormatDialect, FormatOccurrence, FormatOwnedBytes,
    FormatParseIndex, FormatParseResult, FormatParseState, FormatScratch, StringOccurrence,
};
use super::super::model::types::Declarations;
use super::super::nodes::types::Node;
use super::super::semantic_syntax::types::Visibility;
use super::super::syntax::service::{below, interpolation, kind, literal, standing};
use super::super::syntax::types::{Interpolation, Kind, Literal, Standing};
use super::super::tokens::types::Token;

/// Reset current identities and admit small, content-only results for one following source.
pub(crate) fn clear(index: &mut FormatParseIndex) {
    crate::domain::grammar_cache::service::empty(&mut index.memo);
    for (input, at) in index.by_input.drain(..) {
        if let Some(value) = index.values.get_mut(at.0) {
            let result = std::mem::replace(&mut value.result, FormatParseResult::Unavailable);
            let bytes = owned_bytes(&result);
            if result != FormatParseResult::Unavailable {
                crate::domain::grammar_cache::service::admit(
                    &mut index.memo,
                    input.0,
                    result,
                    bytes.0,
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

/// Owned child capacity of a grammar result, excluding its inline representation.
#[must_use]
pub(crate) fn owned_bytes(result: &FormatParseResult) -> FormatOwnedBytes {
    FormatOwnedBytes(match result {
        FormatParseResult::Parsed(parsed) => parsed
            .fields
            .capacity()
            .saturating_mul(size_of::<
                crate::domain::format_syntax::types::FormatFieldValue,
            >())
            .saturating_add(
                parsed
                    .fields
                    .iter()
                    .map(|field| match &field.argument {
                        crate::domain::format_syntax::types::FormatArgument::Mapping(key) => {
                            key.0.capacity()
                        }
                        crate::domain::format_syntax::types::FormatArgument::Positional => 0,
                    })
                    .sum::<usize>(),
            ),
        FormatParseResult::Malformed(_) | FormatParseResult::Unavailable => 0,
    })
}

/// Add one occurrence, sharing a prior parse when its decoded value has already appeared.
fn occurrence<P>(tree: &Tree<'_>, index: &mut FormatParseIndex, at: Position)
where
    P: FormatSyntax,
{
    let mut decoded = std::mem::take(&mut index.memo.scratch);
    match decoded_into(tree, at, FormatScratch(&mut decoded)) {
        Ok(DecodedInput::Static) => {}
        Ok(DecodedInput::Dynamic) => {
            crate::domain::grammar_cache::service::scratch(&mut index.memo, decoded);
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
            crate::domain::grammar_cache::service::scratch(&mut index.memo, decoded);
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
        crate::domain::grammar_cache::service::scratch(&mut index.memo, decoded);
        index.occurrences.push(FormatOccurrence {
            node: at,
            cached: Some(cached),
            unknown_input: false.into(),
        });
        return;
    }

    if index.values.try_reserve(1).is_err()
        || index.occurrences.try_reserve(1).is_err()
        || !crate::domain::grammar_cache::service::reserve(&mut index.by_input)
    {
        index.state = FormatParseState::Partial;
        return;
    }
    let (decoded, parsed) = if let Some((key, result)) =
        crate::domain::grammar_cache::service::take(&mut index.memo, &decoded)
    {
        crate::domain::grammar_cache::service::scratch(&mut index.memo, decoded);
        (key, result)
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
pub(crate) fn prepare<'a, P>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Ref<'a, FormatParseIndex>
where
    P: FormatSyntax,
{
    if !crate::domain::grammar_cache::service::is_grammar::<P, _>(
        &model.format_parsing.borrow().memo,
        P::DECODED_GRAMMAR.0,
    ) {
        clear(&mut model.format_parsing.borrow_mut());
    }
    let held: &RefCell<FormatParseIndex> = &model.format_parsing;
    if held.borrow().state == FormatParseState::Fresh {
        let mut index = held.borrow_mut();
        crate::domain::grammar_cache::service::grammar::<P, _>(
            &mut index.memo,
            P::DECODED_GRAMMAR.0,
        );
        if languages::has_incomplete_syntax(tree.elements)
            || languages::has_incomplete_lexical_stream(tree.elements)
        {
            index.state = FormatParseState::Partial;
        } else {
            index.state = FormatParseState::Complete;
        }
        let positions = super::super::semantic_syntax::service::positions(tree);
        for at in positions {
            if is_string_occurrence(tree, at) != StringOccurrence::String
                || standing(tree, at) != Standing::Own
                || super::super::semantic_syntax::service::visibility(tree, model, at, symbols)
                    != Visibility::Visible
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
pub(crate) fn state(index: &FormatParseIndex) -> FormatParseState {
    index.state
}

/// The original visible string occurrences and their cached parsed result indexes.
#[must_use]
pub(crate) fn occurrences(index: &FormatParseIndex) -> &[FormatOccurrence] {
    &index.occurrences
}

/// The cached result for one occurrence, when the occurrence was static.
#[must_use]
pub(crate) fn result<'a>(
    index: &'a FormatParseIndex,
    occurrence: &FormatOccurrence,
) -> Option<&'a FormatParseResult> {
    index
        .values
        .get(occurrence.cached?.0)
        .map(|cached| &cached.result)
}
