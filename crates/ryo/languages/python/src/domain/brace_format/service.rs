//! Lazily parse decoded brace-format strings once per distinct input value.
use super::super::model::types::Declarations;
use super::super::nodes::types::Node;
use super::super::semantic_syntax::types::Visibility;
use super::super::syntax::service::kind;
use super::super::syntax::types::Kind;
use super::interface::BraceFormatSyntax;
use super::types::{BraceFormatIndex, BraceFormatOccurrence, BraceFormatResult, BraceFormatState};
use ryo::languages::{Position, Symbols, Tree};
use std::cell::{Ref, RefCell};

/// Why brace-format rows are partial.
pub(crate) const INCOMPLETE: &str = "brace-format parsing needs complete decoded string inputs";

/// Clear source-specific results while retaining allocation capacity.
pub(crate) fn clear(index: &mut BraceFormatIndex) {
    index.by_input.clear();
    index.values.clear();
    index.occurrences.clear();
    index.state = BraceFormatState::Fresh;
}

fn occurrence<P: BraceFormatSyntax>(tree: &Tree<'_>, index: &mut BraceFormatIndex, at: Position) {
    let decoded = super::super::format_parsing::service::decoded_string(tree, at);
    let (cached, unknown_input) = match decoded {
        Ok(None) => (None, true),
        Err(()) => {
            index.state = BraceFormatState::Partial;
            (None, false)
        }
        Ok(Some(input)) => {
            if let Some(cached) = index.by_input.get(&input).copied() {
                (Some(cached), false)
            } else {
                if index.by_input.try_reserve(1).is_err()
                    || index.values.try_reserve(1).is_err()
                    || index.occurrences.try_reserve(1).is_err()
                {
                    index.state = BraceFormatState::Partial;
                    return;
                }
                let cached = index.values.len();
                let result = P::parse(&input);
                if result == BraceFormatResult::Unavailable {
                    index.state = BraceFormatState::Partial;
                }
                let _prior = index.by_input.insert(input, cached);
                index
                    .values
                    .push(super::types::CachedBraceFormat { result });
                (Some(cached), false)
            }
        }
    };
    if index.occurrences.try_reserve(1).is_err() {
        index.state = BraceFormatState::Partial;
        return;
    }
    index.occurrences.push(BraceFormatOccurrence {
        node: at,
        cached,
        unknown_input,
    });
}

/// Prepare once, and only when a brace-format relation is read.
pub(crate) fn prepare<'a, P: BraceFormatSyntax>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Ref<'a, BraceFormatIndex> {
    let held: &RefCell<BraceFormatIndex> = &model.brace_format;
    if held.borrow().state == BraceFormatState::Fresh {
        let mut index = held.borrow_mut();
        for at in super::super::semantic_syntax::service::positions(tree) {
            if !matches!(kind(tree, at), Kind::Node(Node::Constant | Node::JoinedStr))
                || !super::super::format_parsing::service::is_string_occurrence(tree, at)
                || super::super::semantic_syntax::service::visibility(tree, model, at, symbols)
                    != Visibility::Visible
            {
                continue;
            }
            occurrence::<P>(tree, &mut index, at);
        }
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
        .get(occurrence.cached?)
        .map(|cached| &cached.result)
}
