//! The `flow` service's cases, on values alone: a graph built by hand and cleared, and each kind of edge and step said in the words
//! of the flow relations.

use super::service::{clear_flow, flow_edge_word, flow_step_word};
use super::types::{
    BlockNumber, EdgeOrdinal, FlowBlockFact, FlowEdgeFact, FlowEdgeKind, FlowGraph, FlowMemberFact,
    FlowStepFact, FlowStepKind, StepOrdinal,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{Flag, Position};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// A graph of one block, edge, step and member, cleared: how many of them it keeps.
fn a_cleared_graph_writes_no_row() -> usize {
    let mut graph = FlowGraph {
        blocks: vec![FlowBlockFact {
            reachable: Flag::Raised,
            live: Flag::Raised,
            implicit_return: Flag::Lowered,
        }],
        edges: vec![FlowEdgeFact {
            source: BlockNumber(0),
            target: BlockNumber(0),
            kind: FlowEdgeKind::Backedge,
            ordinal: EdgeOrdinal(0),
        }],
        steps: vec![FlowStepFact {
            block: BlockNumber(0),
            ordinal: StepOrdinal(0),
            kind: FlowStepKind::Statement,
            node: Position(1),
        }],
        members: vec![FlowMemberFact {
            node: Position(1),
            block: BlockNumber(0),
        }],
    };
    clear_flow(&mut graph);
    graph.blocks.len() + graph.edges.len() + graph.steps.len() + graph.members.len()
}

/// Every kind of edge, in the order `flow-edge.edge-kind` lists them: 1 when each is said in its word.
fn each_edge_kind_has_its_catalog_word() -> usize {
    let kinds = [
        FlowEdgeKind::Normal,
        FlowEdgeKind::Jump,
        FlowEdgeKind::Backedge,
        FlowEdgeKind::Join,
        FlowEdgeKind::Finalize,
        FlowEdgeKind::NewFunction,
        FlowEdgeKind::Unreachable,
        FlowEdgeKind::ErrorExplicit,
        FlowEdgeKind::ErrorImplicit,
    ];
    let words = kinds.map(|kind| flow_edge_word(kind).0);
    usize::from(
        words
            == [
                "normal",
                "jump",
                "backedge",
                "join",
                "finalize",
                "new-function",
                "unreachable",
                "error-explicit",
                "error-implicit",
            ],
    )
}

/// Every kind of step, in the order `flow-step.step-kind` lists them: 1 when each is said in its word.
fn each_step_kind_has_its_catalog_word() -> usize {
    let kinds = [
        FlowStepKind::Statement,
        FlowStepKind::Return,
        FlowStepKind::ReturnUndefined,
        FlowStepKind::Break,
        FlowStepKind::BreakLabeled,
        FlowStepKind::Continue,
        FlowStepKind::ContinueLabeled,
        FlowStepKind::Throw,
        FlowStepKind::Condition,
        FlowStepKind::IterationOf,
        FlowStepKind::IterationIn,
    ];
    let words = kinds.map(|kind| flow_step_word(kind).0);
    usize::from(
        words
            == [
                "statement",
                "return",
                "return-undefined",
                "break",
                "break-labeled",
                "continue",
                "continue-labeled",
                "throw",
                "condition",
                "iteration-of",
                "iteration-in",
            ],
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_cleared_graph_writes_no_row"),
        word: Word::Succeeds,
        input: || a_cleared_graph_writes_no_row,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("each_edge_kind_has_its_catalog_word"),
        word: Word::Succeeds,
        input: || each_edge_kind_has_its_catalog_word,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("each_step_kind_has_its_catalog_word"),
        word: Word::Succeeds,
        input: || each_step_kind_has_its_catalog_word,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
