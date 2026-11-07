//! The test support of the `builders` concept: the builder calls the tests of other crates make to write an element table.

use super::service;
use crate::domain::elements::types::{Elements, TableBuilder};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{KindBits, Offset, Position};

/// Opens a builder over an element table, a symbol table and the source of a file.
pub fn table_builder<'a>(
    elements: &'a mut Elements,
    symbols: &'a mut Symbols,
    source: ByteSlice<'a>,
) -> TableBuilder<'a> {
    service::table_builder(elements, symbols, source)
}

/// Enters a node of a kind at an offset.
pub fn enter_node(builder: &mut TableBuilder<'_>, kind: KindBits, start: Offset) {
    service::enter_node(builder, kind, start);
}

/// Leaves the node entered last, at an offset.
pub fn leave_node(builder: &mut TableBuilder<'_>, end: Offset) {
    service::leave_node(builder, end);
}

/// Writes a token of a kind between two offsets.
pub fn token(builder: &mut TableBuilder<'_>, kind: KindBits, start: Offset, end: Offset) {
    service::token(builder, kind, start, end);
}

/// The identity of the next element the builder will append.
#[must_use]
pub fn builder_position(builder: &mut TableBuilder<'_>) -> Position {
    service::builder_position(builder)
}
