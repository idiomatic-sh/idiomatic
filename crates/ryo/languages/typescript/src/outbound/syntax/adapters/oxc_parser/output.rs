//! Where the adapter's finished projection goes: into the element table the old contract builds, or into the numbered node table the
//! walk contract builds. The projection is the same; what differs is how a table is written and that the numbered table needs each
//! node's role (its field) when the node is entered.

#![cfg_attr(not(test), allow(dead_code, reason = "the walk is linked when step 0 of the ports lands; its tests read it until then"))]

use super::flow::service::written_flow;
use super::patterns::service::written_patterns;
use super::projection::service::write;
use super::types::{Carrier, FlowDraft, NodePositions, Piece, Text};
use crate::domain::fields::numbered::{Family, field_of_child};
use crate::domain::fields::service::field_bits;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use oxc_allocator::Allocator;
use oxc_semantic::Scoping;
use ryo::languages::{self, FieldCode, NodeTableBuilder, Offset, Position, TableBuilder, node_table};

/// What the adapter's reading ends in.
pub(super) trait Output {
    /// Whether the control-flow relations are read, so the graph is built.
    fn needs_flow(&self) -> bool;
    /// Reports what the library said of the file.
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset);
    /// Writes the tree of pieces.
    fn tree(
        &mut self,
        root: &mut Piece,
        carriers: &mut [Carrier<'_>],
        positions: &mut [Option<Position>],
        source: &str,
    );
    /// Writes the control-flow facts of the file.
    fn flow(&mut self, flow: &FlowDraft, positions: &NodePositions);
    /// Writes the parsed patterns of the file.
    fn patterns(
        &mut self,
        allocator: &Allocator,
        source: languages::NameText<'_>,
        carriers: &[Carrier<'_>],
        scoping: &Scoping,
    );
    /// Says whether the file was accepted.
    fn accept(&mut self, acceptance: languages::SyntaxAcceptance);
}

/// The old contract's table.
pub(super) struct Elements<'b, 't>(pub &'b mut TableBuilder<'t>);

impl Output for Elements<'_, '_> {
    fn needs_flow(&self) -> bool {
        self.0.reads_any(&[
            languages::Relation::FlowBlock,
            languages::Relation::FlowEdge,
            languages::Relation::FlowMember,
            languages::Relation::FlowStep,
        ])
    }
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset) {
        languages::diagnose(self.0, languages::ReportText(message), start, end);
    }
    fn tree(
        &mut self,
        root: &mut Piece,
        carriers: &mut [Carrier<'_>],
        positions: &mut [Option<Position>],
        _source: &str,
    ) {
        write(root, carriers, positions, self.0);
    }
    fn flow(&mut self, flow: &FlowDraft, positions: &NodePositions) {
        written_flow(flow, positions, self.0);
    }
    fn patterns(
        &mut self,
        allocator: &Allocator,
        source: languages::NameText<'_>,
        carriers: &[Carrier<'_>],
        scoping: &Scoping,
    ) {
        written_patterns(allocator, source, carriers, scoping, self.0);
    }
    fn accept(&mut self, acceptance: languages::SyntaxAcceptance) {
        languages::set_syntax_acceptance(self.0, acceptance);
    }
}

/// The numbered node table of the walk contract. Its flow facts and pattern trees are not written into it: the per-file view holds
/// what a filler of those relations reads.
pub(super) struct Numbered<'b, 't>(pub &'b mut NodeTableBuilder<'t>);

impl Output for Numbered<'_, '_> {
    fn needs_flow(&self) -> bool {
        false
    }
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset) {
        node_table::diagnose(self.0, start, end, languages::NameText(message));
    }
    fn tree(
        &mut self,
        root: &mut Piece,
        _carriers: &mut [Carrier<'_>],
        _positions: &mut [Option<Position>],
        source: &str,
    ) {
        write_numbered(root, FieldCode(0), source, self.0);
    }
    fn flow(&mut self, _flow: &FlowDraft, _positions: &NodePositions) {}
    fn patterns(
        &mut self,
        _allocator: &Allocator,
        _source: languages::NameText<'_>,
        _carriers: &[Carrier<'_>],
        _scoping: &Scoping,
    ) {
    }
    fn accept(&mut self, acceptance: languages::SyntaxAcceptance) {
        node_table::set_syntax_acceptance(self.0, acceptance);
    }
}

/// Writes the standard tree into the numbered table, giving each child the role its parent's kind and its siblings say.
fn write_numbered(
    node: &mut Piece,
    field: FieldCode,
    source: &str,
    builder: &mut NodeTableBuilder<'_>,
) {
    node.children
        .sort_by_key(|child| (child.span.start, std::cmp::Reverse(child.span.end)));
    let (start, end) = (Offset(node.span.start), Offset(node.span.end));
    match node.kind {
        Kind::Node(holder) => {
            node_table::enter_node(builder, kind_bits(node.kind), field, start);
            for property in &node.properties {
                node_table::node_property(builder, property.id, languages::NameText(&property.value.0));
            }
            let kinds: Vec<Kind> = node.children.iter().map(|child| child.kind).collect();
            let starts: Vec<u32> = node.children.iter().map(|child| child.span.start).collect();
            let ends: Vec<u32> = node.children.iter().map(|child| child.span.end).collect();
            let family = Family { holder, kinds: &kinds, starts: &starts, ends: &ends, source };
            for (at, child) in node.children.iter_mut().enumerate() {
                let role = field_bits(field_of_child(&family, at));
                write_numbered(child, FieldCode(role.0), source, builder);
            }
            node_table::leave_node(builder, end);
        }
        Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => match &node.text {
            Text::Source => node_table::token(builder, kind_bits(node.kind), field, start, end),
            Text::Decoded(text) => node_table::decoded_token(
                builder,
                kind_bits(node.kind),
                field,
                start,
                end,
                languages::NameText(&text.0),
            ),
        },
    }
}
