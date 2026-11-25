//! Where the adapter's one reading goes: into the element table the old contract builds, or into the numbered node table the walk
//! contract builds. What is written is the same; the numbered table needs each node's role (its field) when the node is entered, and
//! a role is a function of the node's kind alone.

use crate::domain::syntax::service::{field_bits, field_of_kind, kind_bits};
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, FieldCode, NodeTableBuilder, Offset, TableBuilder, node_table};

/// What the adapter writes a file's tree through.
pub(super) trait Output {
    /// Opens a node of `kind` that starts at `start`.
    fn enter(&mut self, kind: Kind, start: Offset);
    /// Opens a node of `kind` the parser implied: it has no bytes of its own and no location.
    fn imply(&mut self, kind: Kind);
    /// Gives the node open a property: its number and its value.
    fn property(&mut self, id: languages::PropertyId, value: languages::NameText<'_>);
    /// Closes the node open where it ends.
    fn leave(&mut self, end: Offset);
    /// Writes a token or an extra, a node with no children.
    fn token(&mut self, kind: Kind, start: Offset, end: Offset);
    /// Reports what the library said of the file.
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset);
}

/// The old contract's element table.
pub(super) struct Elements<'b, 't>(pub &'b mut TableBuilder<'t>);

impl Output for Elements<'_, '_> {
    fn enter(&mut self, kind: Kind, start: Offset) {
        languages::enter_node(self.0, kind_bits(kind), start);
    }
    fn imply(&mut self, kind: Kind) {
        languages::enter_implied_node(self.0, kind_bits(kind));
    }
    fn property(&mut self, id: languages::PropertyId, value: languages::NameText<'_>) {
        languages::node_property(self.0, id, value);
    }
    fn leave(&mut self, end: Offset) {
        languages::leave_node(self.0, end);
    }
    fn token(&mut self, kind: Kind, start: Offset, end: Offset) {
        languages::token(self.0, kind_bits(kind), start, end);
    }
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset) {
        languages::diagnose(self.0, languages::ReportText(message), start, end);
    }
}

/// The numbered node table of the walk contract.
pub(super) struct Numbered<'b, 't>(pub &'b mut NodeTableBuilder<'t>);

/// The role a node of `kind` has under its parent, as the code the table holds.
fn field(kind: Kind) -> FieldCode {
    FieldCode(field_bits(field_of_kind(kind)).0)
}

impl Output for Numbered<'_, '_> {
    fn enter(&mut self, kind: Kind, start: Offset) {
        node_table::enter_node(self.0, kind_bits(kind), field(kind), start);
    }
    fn imply(&mut self, kind: Kind) {
        node_table::enter_unlocated_node(self.0, kind_bits(kind), field(kind));
    }
    fn property(&mut self, id: languages::PropertyId, value: languages::NameText<'_>) {
        node_table::node_property(self.0, id, value);
    }
    fn leave(&mut self, end: Offset) {
        node_table::leave_node(self.0, end);
    }
    fn token(&mut self, kind: Kind, start: Offset, end: Offset) {
        node_table::token(self.0, kind_bits(kind), field(kind), start, end);
    }
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset) {
        node_table::diagnose(self.0, start, end, languages::NameText(message));
    }
}
