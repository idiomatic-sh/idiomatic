//! Where the adapter's finished projection goes: into the element table the old contract builds, or into the numbered node table the
//! walk contract builds. The projection is the same; what differs is how a table is written and that the numbered table needs each
//! node's role (its field) when the node is entered.

#![cfg_attr(not(test), allow(dead_code, reason = "the walk is linked when step 0 of the ports lands; its tests read it until then"))]

use super::flow::service::written_flow;
use super::patterns::service::{written_numbered_patterns, written_patterns};
use super::projection::service::write;
use super::types::{Carrier, Carries, FlowDraft, Hosting, NodePositions, Piece, Text};
use crate::domain::fields::numbered::{Family, field_of_child};
use crate::domain::fields::service::field_bits;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use oxc_allocator::Allocator;
use oxc_semantic::Scoping;
use crate::domain::file_view::types::{CarrierFacts, FlowFacts};
use crate::outbound::syntax::types::Wants;
use ryo::languages::{self, FieldCode, FlowMemberFact, FlowStepFact, NodeTableBuilder, Offset, Position, TableBuilder, node_table};

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

/// The numbered node table of the walk contract. Its pattern trees are not written into it: the per-file view holds what a filler of
/// those relations reads. Its flow facts are kept for the fillers, each node as its id in the table.
pub(super) struct Numbered<'b, 't> {
    /// The table written.
    builder: &'b mut NodeTableBuilder<'t>,
    /// What the fillers want kept.
    wants: Wants,
    /// The control-flow facts, once written.
    pub(super) flow: Option<FlowFacts>,
    /// What the library made of each written carrier's pattern.
    pub(super) patterns: Vec<CarrierFacts>,
}

impl<'b, 't> Numbered<'b, 't> {
    /// An output over the table `builder` writes, with the control-flow facts wanted or not.
    pub(super) fn new(builder: &'b mut NodeTableBuilder<'t>, wants: Wants) -> Self {
        Self { builder, wants, flow: None, patterns: Vec::new() }
    }
}

impl Output for Numbered<'_, '_> {
    fn needs_flow(&self) -> bool {
        self.wants.flow
    }
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset) {
        node_table::diagnose(self.builder, start, end, languages::NameText(message));
    }
    fn tree(
        &mut self,
        root: &mut Piece,
        carriers: &mut [Carrier<'_>],
        positions: &mut [Option<Position>],
        source: &str,
    ) {
        let mut nodes = 0_u32;
        write_numbered(root, FieldCode(0), source, positions, carriers, &mut nodes, self.builder);
    }
    fn flow(&mut self, flow: &FlowDraft, positions: &NodePositions) {
        let mut facts = FlowFacts { blocks: flow.blocks.clone(), edges: flow.edges.clone(), ..FlowFacts::default() };
        for drafted in &flow.steps {
            if let Some(node) = positions.0.get(drafted.node.index()).copied().flatten() {
                facts.steps.push(FlowStepFact { node, ..drafted.step });
            }
        }
        for (position, block) in positions.0.iter().zip(&flow.members) {
            if let Some(node) = *position {
                facts.members.push(FlowMemberFact { node, block: *block });
            }
        }
        self.flow = Some(facts);
    }
    fn patterns(
        &mut self,
        allocator: &Allocator,
        source: languages::NameText<'_>,
        carriers: &[Carrier<'_>],
        scoping: &Scoping,
    ) {
        if self.wants.patterns {
            self.patterns = written_numbered_patterns(allocator, source, carriers, scoping, self.builder);
        }
    }
    fn accept(&mut self, acceptance: languages::SyntaxAcceptance) {
        node_table::set_syntax_acceptance(self.builder, acceptance);
    }
}

/// Writes the standard tree into the numbered table, giving each child the role its parent's kind and its siblings say.
fn write_numbered(
    node: &mut Piece,
    field: FieldCode,
    source: &str,
    positions: &mut [Option<Position>],
    carriers: &mut [Carrier<'_>],
    nodes: &mut u32,
    builder: &mut NodeTableBuilder<'_>,
) {
    if let Some(id) = node.node
        && let Some(position) = positions.get_mut(id.index())
        && matches!(node.kind, Kind::Node(_))
    {
        // The nodes of the numbered table come first, in pre-order: a node's id is how many nodes were entered before it.
        *position = Some(Position(*nodes));
    }
    node.children
        .sort_by_key(|child| (child.span.start, std::cmp::Reverse(child.span.end)));
    let (start, end) = (Offset(node.span.start), Offset(node.span.end));
    match node.kind {
        Kind::Node(holder) => {
            if let Carries::Pattern(index) = node.carries
                && let Some(carrier) = carriers.get_mut(index.0)
            {
                carrier.host = Hosting::At(Position(*nodes));
            }
            *nodes += 1;
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
                write_numbered(child, FieldCode(role.0), source, positions, carriers, nodes, builder);
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
