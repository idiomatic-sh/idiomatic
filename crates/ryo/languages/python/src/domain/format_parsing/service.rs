//! Lazily cache the decoded grammar result once per distinct source string.
use std::cell::{Ref, RefCell};

use ryo::languages::{self, Position, Symbols, Tree};

use super::super::format_syntax::interface::FormatSyntax;
use super::super::format_syntax::types::{
    CachedFormat, FormatDialect, FormatOccurrence, FormatParseIndex, FormatParseResult,
    FormatParseState,
};
use super::super::model::types::Declarations;
use super::super::nodes::types::Node;
use super::super::semantic_syntax::types::Visibility;
use super::super::syntax::service::{below, interpolation, kind, literal, standing};
use super::super::syntax::types::{Interpolation, Kind, Literal, Standing};
use super::super::tokens::types::Token;

/// Why the format relations are only partly known.
pub(crate) const INCOMPLETE: &str =
    "format parsing needs complete decoded string inputs and source syntax";

/// Clear current values while retaining collection capacities for the next source.
pub(crate) fn clear(index: &mut FormatParseIndex) {
    index.by_input.clear();
    index.values.clear();
    index.occurrences.clear();
    index.state = FormatParseState::Fresh;
}

/// Whether this is one original visible string-literal node.
pub(crate) fn is_string_occurrence(tree: &Tree<'_>, at: Position) -> bool {
    matches!(kind(tree, at), Kind::Node(Node::JoinedStr))
        || (kind(tree, at) == Kind::Node(Node::Constant) && literal(tree, at) == Literal::String)
}

/// Concatenate parser-decoded string pieces. Dynamic interpolations are known unknown inputs;
/// missing decoded parser payload is unavailable and must make the relation partial.
pub(crate) fn decoded_string(tree: &Tree<'_>, at: Position) -> Result<Option<String>, ()> {
    if interpolation(tree, at) == Interpolation::Interpolates {
        return Ok(None);
    }
    let mut text = String::new();
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
    Ok(Some(text))
}

/// Add one occurrence, sharing a prior parse when its decoded value has already appeared.
fn occurrence<P: FormatSyntax>(tree: &Tree<'_>, index: &mut FormatParseIndex, at: Position) {
    let decoded = match decoded_string(tree, at) {
        Ok(Some(decoded)) => decoded,
        Ok(None) => {
            if index.occurrences.try_reserve(1).is_err() {
                index.state = FormatParseState::Partial;
                return;
            }
            index.occurrences.push(FormatOccurrence {
                node: at,
                cached: None,
                unknown_input: true,
            });
            return;
        }
        Err(()) => {
            index.state = FormatParseState::Partial;
            if index.occurrences.try_reserve(1).is_ok() {
                index.occurrences.push(FormatOccurrence {
                    node: at,
                    cached: None,
                    unknown_input: false,
                });
            }
            return;
        }
    };

    if let Some(cached) = index.by_input.get(decoded.as_str()).copied() {
        if index.occurrences.try_reserve(1).is_err() {
            index.state = FormatParseState::Partial;
            return;
        }
        index.occurrences.push(FormatOccurrence {
            node: at,
            cached: Some(cached),
            unknown_input: false,
        });
        return;
    }

    if index.values.try_reserve(1).is_err()
        || index.occurrences.try_reserve(1).is_err()
        || index.by_input.try_reserve(1).is_err()
    {
        index.state = FormatParseState::Partial;
        return;
    }
    let parsed = P::parse(&decoded, FormatDialect::PercentString);
    if parsed == FormatParseResult::Unavailable {
        index.state = FormatParseState::Partial;
    }
    let cached = index.values.len();
    index.values.push(CachedFormat { result: parsed });
    let _prior = index.by_input.insert(decoded, cached);
    index.occurrences.push(FormatOccurrence {
        node: at,
        cached: Some(cached),
        unknown_input: false,
    });
}

/// Build the string occurrence index and parse each distinct decoded input at most once.
pub(crate) fn prepare<'a, P: FormatSyntax>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Ref<'a, FormatParseIndex> {
    let held: &RefCell<FormatParseIndex> = &model.format_parsing;
    if held.borrow().state == FormatParseState::Fresh {
        let mut index = held.borrow_mut();
        if languages::has_incomplete_syntax(tree.elements)
            || languages::has_incomplete_lexical_stream(tree.elements)
        {
            index.state = FormatParseState::Partial;
        } else {
            index.state = FormatParseState::Complete;
        }
        let positions = super::super::semantic_syntax::service::positions(tree);
        for at in positions {
            if !is_string_occurrence(tree, at)
                || standing(tree, at) != Standing::Own
                || super::super::semantic_syntax::service::visibility(tree, model, at, symbols)
                    != Visibility::Visible
            {
                continue;
            }
            occurrence::<P>(tree, &mut index, at);
        }
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
        .get(occurrence.cached?)
        .map(|cached| &cached.result)
}
