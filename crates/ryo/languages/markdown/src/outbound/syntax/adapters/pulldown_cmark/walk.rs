//! The node table the walk writes: the tree of the writer's events, numbered as the engine's table numbers it, each node and token
//! with its role under the construct open.

use super::adapter::fill;
use super::interface::{Ops, Tree};
use super::types::Gap;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::file_view::service::record;
use crate::domain::syntax::service::{field_between, field_bits, kind_bits};
use crate::domain::syntax::types::Kind;
use ryo::languages::{FieldCode, NodeTableBuilder, Offset, node_table};

/// The tree written into the builder of a file's node table, and where its last token ends.
pub(super) struct Numbered<'b, 'w, 'd> {
    /// What the tree is written through.
    pub builder: &'b mut NodeTableBuilder<'w>,
    /// Where the last token written ends.
    pub end: Offset,
    /// The nodes open, the document first.
    open: Vec<Kind>,
    /// What the library decoded of the tokens, kept beside the table.
    decodings: &'d mut crate::domain::file_view::types::Decodings,
}

impl<'b, 'w, 'd> Numbered<'b, 'w, 'd> {
    /// A tree written through `builder`, whose document node is open.
    pub(super) fn new(builder: &'b mut NodeTableBuilder<'w>, decodings: &'d mut crate::domain::file_view::types::Decodings) -> Self {
        Self {
            decodings,
            builder,
            end: Offset(0),
            open: vec![Kind::Node(Node::Root)],
        }
    }

    /// The role `kind` has under the node open.
    fn field(&self, kind: Kind) -> FieldCode {
        let field = self
            .open
            .last()
            .map_or(crate::domain::syntax::types::Field::None, |parent| field_between(*parent, kind));
        FieldCode(field_bits(field).0)
    }
}

impl Ops for Numbered<'_, '_, '_> {
    fn token(&mut self, kind: Kind, from: Offset, to: Offset) {
        let field = self.field(kind);
        node_table::token(self.builder, kind_bits(kind), field, from, to);
    }

    fn decoded(&mut self, kind: Kind, from: Offset, to: Offset, text: &str) {
        let field = self.field(kind);
        node_table::token(self.builder, kind_bits(kind), field, from, to);
        record(self.decodings, from, to, text);
    }
}

impl Tree for Numbered<'_, '_, '_> {
    fn enter(&mut self, kind: Kind, start: Offset, gap: &mut Gap<'_>) {
        self.fill_to(start, gap);
        let field = self.field(kind);
        node_table::enter_node(self.builder, kind_bits(kind), field, start.max(self.end));
        self.open.push(kind);
    }

    fn leave(&mut self) {
        node_table::leave_node(self.builder, self.end);
        let _: Option<Kind> = self.open.pop();
    }

    fn fill_to(&mut self, to: Offset, gap: &mut Gap<'_>) {
        if to > self.end {
            let from = self.end;
            fill(gap, self, from, to);
            self.end = to;
        }
    }

    fn token(&mut self, kind: Kind, start: Offset, end: Offset, gap: &mut Gap<'_>) {
        self.fill_to(start, gap);
        let start = start.max(self.end);
        let end = end.max(start);
        Ops::token(self, kind, start, end);
        self.end = end;
    }

    fn decoded(&mut self, kind: Kind, start: Offset, end: Offset, text: &str, gap: &mut Gap<'_>) {
        self.fill_to(start, gap);
        let start = start.max(self.end);
        let end = end.max(start);
        Ops::decoded(self, kind, start, end, text);
        self.end = end;
    }

    fn finish(&mut self, read: Offset, length: Offset, gap: &mut Gap<'_>) {
        self.fill_to(read, gap);
        if length > self.end {
            let from = self.end;
            Ops::token(self, Kind::Extra(Extra::Error), from, length);
            self.end = length;
        }
        node_table::leave_node(self.builder, length);
    }
}
