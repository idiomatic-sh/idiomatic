//! The `model` service: it answers each reader of a file from the file's model, which `documents` works out once when the file is
//! parsed and the engine gives every reader (`Parse::FileModel`): a reader looks its element up, and nothing is scanned again.

use super::constants::{DECIMAL, RADIX_PREFIXES};
use super::types::{ElementFact, EntryFact, MappingFact, Model, Readability, Stringness};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, parent, value_kind, written};
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, Position, Tree, ValueKind};

/// The mapping the element at `at` first opens, when it opens one.
#[must_use]
pub(crate) fn mapping_at(model: &Model, at: Position) -> Option<MappingFact> {
    match model.mappings.binary_search_by_key(&at, |each| each.id) {
        Ok(found) => model.mappings.get(found).copied(),
        // The search answers where the position would go: no fact is at it.
        Err(_) => None,
    }
}

/// The entry the part of a key at `at` is, when it opens one.
#[must_use]
pub(crate) fn entry_at(model: &Model, at: Position) -> Option<EntryFact> {
    match model
        .entries
        .binary_search_by_key(&at, |each| each.occurrence)
    {
        Ok(found) => model.entries.get(found).copied(),
        // The search answers where the position would go: no fact is at it.
        Err(_) => None,
    }
}

/// The element of a sequence the element at `at` is, when it is one.
#[must_use]
pub(crate) fn element_at(model: &Model, at: Position) -> Option<ElementFact> {
    match model
        .elements
        .binary_search_by_key(&at, |each| each.occurrence)
    {
        Ok(found) => model.elements.get(found).copied(),
        // The search answers where the position would go: no fact is at it.
        Err(_) => None,
    }
}

/// Whether an entry's value could be read.
#[must_use]
pub(crate) fn readability(fact: EntryFact) -> Readability {
    fact.readability
}

/// What the token at `at` says as the library decoded it, as a name; nothing for a token it did not decode, or refused.
pub(crate) fn decoded(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Option<languages::Symbol> {
    let text = languages::decoded_at(tree.elements, at)?;
    let name = languages::begin_name(symbols);
    languages::push_text(symbols, text);
    Some(languages::end_name(symbols, tree.source, name))
}

/// What the string value at `at` says, as the library decoded it; no name for a value that is no string, or one it refused.
pub(crate) fn said(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Option<Position>,
) -> languages::Symbol {
    at.filter(|each| value_kind(kind(tree, *each)) == Some(ValueKind::String))
        .and_then(|each| decoded(tree, symbols, each))
        .unwrap_or(languages::NO_SYMBOL)
}

/// The value at `at` as a yes or no when it is a boolean the library decoded: raised for `true`; lowered for `false`, for every
/// other kind and for no value.
#[must_use]
pub(crate) fn boolean(tree: &Tree<'_>, at: Option<Position>) -> languages::Flag {
    let raised = at
        .filter(|each| value_kind(kind(tree, *each)) == Some(ValueKind::Boolean))
        .and_then(|each| languages::decoded_at(tree.elements, each))
        .and_then(|text| text.0.parse::<bool>().ok()) // a decoded value that does not read as the kind its node says is no value, which the column holds as its default
        .unwrap_or_default();
    if raised {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The value at `at` as a whole number when it is an integer the library decoded; zero for every other kind and for no value.
#[must_use]
pub(crate) fn integer(tree: &Tree<'_>, at: Option<Position>) -> languages::Integer {
    languages::Integer(
        at.filter(|each| value_kind(kind(tree, *each)) == Some(ValueKind::Integer))
            .and_then(|each| languages::decoded_at(tree.elements, each))
            .and_then(|text| text.0.parse::<i64>().ok()) // a decoded value that does not read as the kind its node says is no value, which the column holds as its default
            .unwrap_or_default(),
    )
}

/// The value at `at` as the library decoded it when it is of `wanted` kind, a float or a date-time, as text; none for every other kind
/// and for no value.
#[must_use]
pub(crate) fn decoded_as(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Option<Position>,
    wanted: ValueKind,
) -> languages::Symbol {
    at.filter(|each| value_kind(kind(tree, *each)) == Some(wanted))
        .and_then(|each| decoded(tree, symbols, each))
        .unwrap_or(languages::NO_SYMBOL)
}

/// The base the integer at `at` is written in, by its prefix as the TOML grammar writes it (`0x`, `0o`, `0b`, else a decimal); zero
/// for every other kind and for no value.
#[must_use]
pub(crate) fn radix(tree: &Tree<'_>, at: Option<Position>) -> languages::Integer {
    languages::Integer(
        at.filter(|each| value_kind(kind(tree, *each)) == Some(ValueKind::Integer))
            .map_or(0, |each| {
                let text = written(tree, each).0;
                RADIX_PREFIXES
                    .iter()
                    .find(|(prefix, _)| text.starts_with(prefix))
                    .map_or(DECIMAL, |(_, base)| *base)
            }),
    )
}

/// Whether the string at `at` is a value or a part of a key.
#[must_use]
pub(crate) fn stringness(tree: &Tree<'_>, at: Position) -> Stringness {
    if kind(tree, parent(tree, at)) != Kind::Node(Node::Key)
        && value_kind(kind(tree, at)) == Some(ValueKind::String)
    {
        Stringness::Value
    } else {
        Stringness::Other
    }
}
