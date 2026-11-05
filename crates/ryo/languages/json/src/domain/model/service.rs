//! The `model` service: it works out the places of a file's members and values once, when the file is parsed, and answers a
//! reader the place of an element from them.

use super::constants::PLACE_OF_ELEMENT;
use ryo::languages::{self, Tree};

use super::types::{Counted, Model};
use crate::domain::nodes::types::JsonNode;
use crate::domain::syntax::constants::DECIMAL;
use crate::domain::syntax::service::{is_value, kind, parent, value_kind, written};
use crate::domain::syntax::types::NameWriter;
use crate::domain::syntax::types::{Kind, Valueness};
use ryo::languages::ValueKind;
use std::fmt::Write as _;

/// The place of the element at `at` among the members of its object or the values of its array; zero for an element that is
/// neither.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the model holds a place for every element of its file; an element without one is a bug that must crash, not a place of zero"
)]
pub(crate) fn place(model: &Model, at: languages::Position) -> languages::Integer {
    model
        .places
        .get(index(at).0)
        .copied()
        .expect(PLACE_OF_ELEMENT)
}

/// Works out the places of a file once: every member and every value of a container, counted from zero in the order they are written.
pub(crate) fn places(tree: &Tree<'_>, model: &mut Model) {
    model.places.clear();
    model.places.resize(
        index(languages::end_of(tree.elements)).0,
        languages::Integer(0),
    );
    for container in languages::positions(tree.elements) {
        let counting = match kind(tree, container) {
            Kind::Node(JsonNode::Object) => Counted::Members,
            Kind::Node(JsonNode::Array) => Counted::Values,
            Kind::Node(JsonNode::JsonText | JsonNode::Member) | Kind::Token(_) | Kind::Extra(_) => continue,
        };
        let mut next = languages::Integer(0);
        for child in languages::children(tree.elements, container) {
            let counts = match counting {
                Counted::Members => kind(tree, child) == Kind::Node(JsonNode::Member),
                Counted::Values => is_value(kind(tree, child)) == Valueness::Value,
            };
            if counts && let Some(slot) = model.places.get_mut(index(child).0) {
                *slot = next;
                next = languages::Integer(next.0 + 1);
            }
        }
    }
}

/// The index of a position in the lists that hold one item for each element.
fn index(at: languages::Position) -> languages::Count {
    languages::Count(usize::try_from(at.0).unwrap_or(usize::MAX))
}

/// The place of a mapping in a sequence of mappings: its place in its array, and zero for any other.
#[must_use]
pub(crate) fn mapping_ordinal(
    tree: &Tree<'_>,
    model: &Model,
    object: languages::Position,
) -> languages::Integer {
    if kind(tree, parent(tree, object)) == Kind::Node(JsonNode::Array) {
        place(model, object)
    } else {
        languages::Integer(0)
    }
}

/// The value at `at` as a number when it is a float, in the shortest decimal form that reads back as it (`1e3` is `1000.0`): the
/// library hands a number over as the text it scanned, which RFC 8259's grammar reads as a decimal number. Empty for a value of every
/// other kind.
pub(crate) fn float(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    if value_kind(tree, at) != Some(ValueKind::Float) {
        return languages::NO_SYMBOL;
    }
    let Ok(number) = written(tree, at).0.parse::<f64>() else {
        return languages::NO_SYMBOL;
    };
    let name = languages::begin_name(symbols);
    // `Debug` writes an `f64` in the shortest decimal form that reads back as it, an exponent where one is shorter; `Display` never
    // writes one.
    #[expect(
        clippy::use_debug,
        reason = "`Debug` is the shortest round-trip form of an `f64`, which the column is defined to hold"
    )]
    if write!(NameWriter(symbols), "{number:?}").is_err() {
        return languages::NO_SYMBOL;
    }
    languages::end_name(symbols, tree.source, name)
}

/// The base the value at `at` is written in when it is an integer: ten, the only base of RFC 8259's grammar. Zero for a value of
/// every other kind.
#[must_use]
pub(crate) fn radix(tree: &Tree<'_>, at: languages::Position) -> languages::Integer {
    languages::Integer(if value_kind(tree, at) == Some(ValueKind::Integer) {
        DECIMAL
    } else {
        0
    })
}
