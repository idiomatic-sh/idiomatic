//! The node table the walk writes: the tree of the writer's events, numbered as the engine's table numbers it, each node and token
//! with its role under the construct open.

use super::writer::interface::Sink;
use crate::domain::extras::types::Extra;
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
    /// The constructs open, the document first.
    pub open: Vec<Kind>,
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
            open: vec![Kind::Node(crate::domain::nodes::types::Node::Toml)],
        }
    }

    /// The role `kind` has under the construct open.
    fn field(&self, kind: Kind) -> FieldCode {
        let parent = self.open.last().copied();
        FieldCode(field_bits(parent.map_or(crate::domain::syntax::types::Field::None, |parent| field_between(parent, kind))).0)
    }

    /// Writes the text no event spans before `to` as text the parser could not read.
    fn fill_to(&mut self, to: Offset) {
        if to > self.end {
            let field = self.field(Kind::Extra(Extra::Error));
            node_table::token(
                self.builder,
                kind_bits(Kind::Extra(Extra::Error)),
                field,
                self.end,
                to,
            );
            self.end = to;
        }
    }
}

impl Sink for Numbered<'_, '_, '_> {
    fn end(&self) -> Offset {
        self.end
    }

    fn enter(&mut self, kind: Kind, at: Offset) {
        self.fill_to(at);
        let field = self.field(kind);
        node_table::enter_node(self.builder, kind_bits(kind), field, at.max(self.end));
        self.open.push(kind);
    }

    fn leave(&mut self) {
        let end = self.end;
        node_table::leave_node(self.builder, end);
        let _: Option<Kind> = self.open.pop();
    }

    fn finish(&mut self, length: Offset) {
        self.fill_to(length);
        node_table::leave_node(self.builder, length);
    }

    fn token(&mut self, kind: Kind, start: Offset, end: Offset) {
        self.fill_to(start);
        let start = start.max(self.end);
        let end = end.max(start);
        let field = self.field(kind);
        node_table::token(self.builder, kind_bits(kind), field, start, end);
        self.end = end;
    }

    fn decoded(&mut self, kind: Kind, start: Offset, end: Offset, text: &str) {
        self.fill_to(start);
        let start = start.max(self.end);
        let end = end.max(start);
        let field = self.field(kind);
        node_table::token(self.builder, kind_bits(kind), field, start, end);
        record(self.decodings, start, end, text);
        self.end = end;
    }
}
