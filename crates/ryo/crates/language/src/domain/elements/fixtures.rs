//! The test support of the `elements` concept: the reads of an element table the tests of other crates make.

use super::service;
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{Count, Position};
use ryo_relations::domain::terms::types::Symbol;

/// The symbols that name the kinds of a table.
#[must_use]
pub fn kind_names(elements: &Elements) -> &[Symbol] {
    service::kind_names(elements)
}

/// How many real elements a table holds.
#[must_use]
pub fn real_count(elements: &Elements) -> Count {
    service::real_count(elements)
}

/// The symbol that is the text at a position.
#[must_use]
pub fn text_at(elements: &Elements, at: Position) -> Symbol {
    service::text_at(elements, at)
}
