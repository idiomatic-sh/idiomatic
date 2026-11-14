//! The control flow of the adapter: the blocks, edges and steps of the graph `oxc_semantic` builds with its `cfg` feature, and the
//! block each node is evaluated in, drafted by the parser's node ids before the semantic pass is dropped and kept in the table by
//! element position once the projection has written every element.

use super::super::types::{DraftStep, FlowDraft, NodePositions, Piece};
use oxc_ast::AstKind;
use oxc_cfg::graph::Direction;
use oxc_cfg::graph::visit::{Control, DfsEvent, EdgeRef as _, set_depth_first_search};
use oxc_cfg::{
    BlockNodeId, ControlFlowGraph, EdgeType, ErrorEdgeKind, EvalConstConditionResult, Instruction,
    InstructionKind, IterationInstructionKind, LabeledInstruction, ReturnInstructionKind,
};
use oxc_semantic::{AstNodes, Semantic};
use ryo::languages::{
    self, BlockNumber, Flag, FlowBlockFact, FlowEdgeFact, FlowEdgeKind, FlowMemberFact,
    FlowStepFact, FlowStepKind, Position, StepOrdinal, TableBuilder,
};

/// Drafts the graphs of the file the semantic pass read: every block with its marks, every edge, every step with a node, and every
/// node's block, by the parser's ids. `live` is the reach of control once a loop whose test is the literal `true` exits only through
/// a break, as oxlint's walk finds it with oxc's `is_infinite_loop_start`.
pub(in super::super) fn drafted_flow(semantic: &Semantic<'_>, draft: &mut FlowDraft) {
    let Some(cfg) = semantic.cfg() else {
        return;
    };
    let graph = cfg.graph();
    let live = live_blocks(semantic.nodes(), cfg);
    for (block, live) in graph.node_indices().zip(live) {
        let basic = cfg.basic_block(block);
        let instructions = basic.instructions();
        let ends = instructions
            .iter()
            .any(|instruction| matches!(instruction.kind, InstructionKind::ImplicitReturn));
        draft.blocks.push(FlowBlockFact {
            reachable: if basic.is_unreachable() {
                Flag::Lowered
            } else {
                Flag::Raised
            },
            live,
            implicit_return: if ends { Flag::Raised } else { Flag::Lowered },
        });
        for (ordinal, instruction) in instructions.iter().enumerate() {
            if let (Some(kind), Some(node)) = (step_kind(instruction.kind), instruction.node_id) {
                draft.steps.push(DraftStep {
                    step: FlowStepFact {
                        block: number(block),
                        ordinal: StepOrdinal(u32::try_from(ordinal).unwrap_or(u32::MAX)),
                        kind,
                        node: Position(0),
                    },
                    node,
                });
            }
        }
    }
    draft
        .edges
        .extend(graph.edge_references().map(|edge| FlowEdgeFact {
            source: number(edge.source()),
            target: number(edge.target()),
            kind: edge_kind(edge.weight()),
        }));
    let nodes = semantic.nodes();
    draft
        .members
        .extend(nodes.iter().map(|node| number(nodes.cfg_id(node.id()))));
}

/// Numbers the pieces of the written tree in the preorder `write` gave them their elements in, from `first`, one element for each
/// piece, and keeps the position of each piece the parser gave a node id.
pub(in super::super) fn placed_nodes(root: &Piece, first: Position, positions: &mut NodePositions) {
    let mut next = first;
    let mut pending = vec![root];
    while let Some(piece) = pending.pop() {
        if let Some(node) = piece.node
            && let Some(slot) = positions.0.get_mut(node.index())
        {
            *slot = Some(next);
        }
        next = Position(next.0 + 1);
        pending.extend(piece.children.iter().rev());
    }
}

/// Keeps the drafted graphs in the table, each node id turned into the position the projection wrote its element at; a step or a
/// node with no element of its own keeps no row.
pub(in super::super) fn written_flow(
    draft: &FlowDraft,
    positions: &NodePositions,
    builder: &mut TableBuilder<'_>,
) {
    // The table numbers the blocks from zero in the order given, as oxc numbers them, so the drafted edges and steps name them as
    // they are.
    for block in &draft.blocks {
        let _: BlockNumber = languages::add_flow_block(builder, *block);
    }
    for edge in &draft.edges {
        languages::add_flow_edge(builder, *edge);
    }
    for drafted in &draft.steps {
        if let Some(node) = positions.0.get(drafted.node.index()).copied().flatten() {
            languages::add_flow_step(
                builder,
                FlowStepFact {
                    node,
                    ..drafted.step
                },
            );
        }
    }
    for (position, block) in positions.0.iter().zip(&draft.members) {
        if let Some(node) = *position {
            languages::add_flow_member(
                builder,
                FlowMemberFact {
                    node,
                    block: *block,
                },
            );
        }
    }
}

/// Whether control reaches each block, by block index: lowered for oxc's own unreachable mark, and for every block after a loop
/// whose test is the literal `true` that only an edge from inside the loop leads to. Ported from oxlint 1.76.0's
/// `effective_unreachable_blocks` (`oxc_linter/src/utils/control_flow.rs`), each of its `unreachable` marks the lowered flag here;
/// the answer depends on its walk order, which this keeps by calling the same `oxc_cfg` walk. `oxc_linter` is not published.
fn live_blocks(nodes: &AstNodes<'_>, cfg: &ControlFlowGraph) -> Vec<Flag> {
    let graph = cfg.graph();
    let mut live = vec![Flag::Lowered; cfg.basic_blocks.len()];
    let mut infinite_loops = Vec::new();
    for node in graph.node_indices() {
        let is_unreachable = cfg.basic_block(node).is_unreachable();
        if let Some(slot) = live.get_mut(node.index()) {
            *slot = if is_unreachable {
                Flag::Lowered
            } else {
                Flag::Raised
            };
        }
        if !is_unreachable
            && let Some(found) = cfg
                .is_infinite_loop_start(node, |instruction| constant_condition(nodes, instruction))
        {
            infinite_loops.push(found);
        }
    }
    for (_, end) in infinite_loops {
        let starts: Vec<BlockNodeId> = graph
            .edges_directed(end, Direction::Outgoing)
            .filter(|edge| matches!(edge.weight(), EdgeType::Normal))
            .map(|edge| edge.target())
            .collect();
        let _: Control<()> = set_depth_first_search(graph, starts, |event| match event {
            DfsEvent::Discover(node, _) => {
                let kept = graph.edges_directed(node, Direction::Incoming).any(|edge| {
                    keeps_reach(cfg, &live, end, edge.source(), edge.weight()) == Flag::Raised
                });
                if kept {
                    Control::Prune
                } else {
                    if let Some(slot) = live.get_mut(node.index()) {
                        *slot = Flag::Lowered;
                    }
                    Control::Continue
                }
            }
            DfsEvent::TreeEdge(..)
            | DfsEvent::BackEdge(..)
            | DfsEvent::CrossForwardEdge(..)
            | DfsEvent::Finish(..) => Control::Continue,
        });
    }
    live
}

/// Whether an edge of `kind` from `source` into a block after the loop ending at `end` keeps control reaching that block: a new
/// function, a `finally` or a `catch` always does; a fall-through or a jump from a block outside the loop that control reaches does;
/// a jump out of a block that breaks does.
fn keeps_reach(
    cfg: &ControlFlowGraph,
    live: &[Flag],
    end: BlockNodeId,
    source: BlockNodeId,
    kind: &EdgeType,
) -> Flag {
    let keeps = match kind {
        EdgeType::NewFunction | EdgeType::Finalize | EdgeType::Error(ErrorEdgeKind::Explicit) => {
            true
        }
        EdgeType::Normal | EdgeType::Jump
            if source != end && live.get(source.index()) == Some(&Flag::Raised) =>
        {
            true
        }
        EdgeType::Jump => cfg
            .basic_block(source)
            .instructions()
            .iter()
            .any(|instruction| matches!(instruction.kind, InstructionKind::Break(_))),
        EdgeType::Normal
        | EdgeType::Backedge
        | EdgeType::Error(ErrorEdgeKind::Implicit)
        | EdgeType::Unreachable
        | EdgeType::Join => false,
    };
    if keeps { Flag::Raised } else { Flag::Lowered }
}

/// What a loop's test is worth, as oxlint reads it: a boolean literal's value, any other test unknown, no test not found.
fn constant_condition(nodes: &AstNodes<'_>, instruction: &Instruction) -> EvalConstConditionResult {
    match (instruction.kind, instruction.node_id) {
        (InstructionKind::Condition, Some(id)) => {
            if let AstKind::BooleanLiteral(literal) = nodes.kind(id) {
                EvalConstConditionResult::Eval(literal.value)
            } else {
                EvalConstConditionResult::Fail
            }
        }
        _ => EvalConstConditionResult::NotFound,
    }
}

/// The kind of a step that can be an element; none for oxc's implicit return and unreachable mark, which never are.
fn step_kind(kind: InstructionKind) -> Option<FlowStepKind> {
    Some(match kind {
        InstructionKind::Statement => FlowStepKind::Statement,
        InstructionKind::Return(ReturnInstructionKind::NotImplicitUndefined) => {
            FlowStepKind::Return
        }
        InstructionKind::Return(ReturnInstructionKind::ImplicitUndefined) => {
            FlowStepKind::ReturnUndefined
        }
        InstructionKind::Break(LabeledInstruction::Unlabeled) => FlowStepKind::Break,
        InstructionKind::Break(LabeledInstruction::Labeled) => FlowStepKind::BreakLabeled,
        InstructionKind::Continue(LabeledInstruction::Unlabeled) => FlowStepKind::Continue,
        InstructionKind::Continue(LabeledInstruction::Labeled) => FlowStepKind::ContinueLabeled,
        InstructionKind::Throw => FlowStepKind::Throw,
        InstructionKind::Condition => FlowStepKind::Condition,
        InstructionKind::Iteration(IterationInstructionKind::Of) => FlowStepKind::IterationOf,
        InstructionKind::Iteration(IterationInstructionKind::In) => FlowStepKind::IterationIn,
        InstructionKind::ImplicitReturn | InstructionKind::Unreachable => return None,
    })
}

/// The kind of an edge of oxc's graph.
fn edge_kind(kind: &EdgeType) -> FlowEdgeKind {
    match kind {
        EdgeType::Normal => FlowEdgeKind::Normal,
        EdgeType::Jump => FlowEdgeKind::Jump,
        EdgeType::Backedge => FlowEdgeKind::Backedge,
        EdgeType::Join => FlowEdgeKind::Join,
        EdgeType::Finalize => FlowEdgeKind::Finalize,
        EdgeType::NewFunction => FlowEdgeKind::NewFunction,
        EdgeType::Unreachable => FlowEdgeKind::Unreachable,
        EdgeType::Error(ErrorEdgeKind::Explicit) => FlowEdgeKind::ErrorExplicit,
        EdgeType::Error(ErrorEdgeKind::Implicit) => FlowEdgeKind::ErrorImplicit,
    }
}

/// The block an oxc block id is, by its number in the file: oxc numbers its blocks densely from zero, in the order they are drafted.
fn number(block: BlockNodeId) -> BlockNumber {
    BlockNumber(u32::try_from(block.index()).unwrap_or(u32::MAX))
}
