//! The `flow_table` service's cases: a graph kept through a file's builder and read back, numbered from zero, discarded by a builder
//! that discards, and not seen by the next file.

use super::service::{add_flow_block, add_flow_edge, add_flow_member, add_flow_step, flow_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::builders::service::{discarding_builder, table_builder};
use crate::domain::elements::types::{Elements, TableBuilder};
use crate::domain::flow::types::{
    BlockNumber, FlowBlockFact, FlowEdgeFact, FlowEdgeKind, FlowMemberFact, FlowStepFact,
    FlowStepKind, StepOrdinal,
};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{Flag, Position};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// A block whose three marks are `reachable`, `live` and `implicit_return`.
fn block(reachable: Flag, live: Flag, implicit_return: Flag) -> FlowBlockFact {
    FlowBlockFact {
        reachable,
        live,
        implicit_return,
    }
}

/// Writes a graph of three blocks, an edge, a step and a member through `builder`, and answers the numbers the blocks were given.
fn written_graph(builder: &mut TableBuilder<'_>) -> [BlockNumber; 3] {
    let numbers = [
        add_flow_block(builder, block(Flag::Raised, Flag::Raised, Flag::Lowered)),
        add_flow_block(builder, block(Flag::Lowered, Flag::Lowered, Flag::Lowered)),
        add_flow_block(builder, block(Flag::Raised, Flag::Lowered, Flag::Raised)),
    ];
    add_flow_edge(
        builder,
        FlowEdgeFact {
            source: BlockNumber(0),
            target: BlockNumber(2),
            kind: FlowEdgeKind::Jump,
        },
    );
    add_flow_step(
        builder,
        FlowStepFact {
            block: BlockNumber(1),
            ordinal: StepOrdinal(1),
            kind: FlowStepKind::Statement,
            node: Position(0),
        },
    );
    add_flow_member(
        builder,
        FlowMemberFact {
            node: Position(0),
            block: BlockNumber(1),
        },
    );
    numbers
}

/// Three blocks written in turn: 1 when they are numbered 0, 1 and 2 and the graph keeps them, with the edge, step and member, as given.
fn blocks_are_numbered_from_zero_in_the_order_given() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
    let numbers = written_graph(&mut builder);
    let graph = flow_of(&elements);
    usize::from(
        numbers == [BlockNumber(0), BlockNumber(1), BlockNumber(2)]
            && graph.blocks
                == [
                    block(Flag::Raised, Flag::Raised, Flag::Lowered),
                    block(Flag::Lowered, Flag::Lowered, Flag::Lowered),
                    block(Flag::Raised, Flag::Lowered, Flag::Raised),
                ]
            && graph.edges.len() == 1
            && graph.steps.len() == 1
            && graph.members.len() == 1,
    )
}

/// A graph written through a builder that discards: 1 when the table keeps no block, edge, step or member of it.
fn a_discarding_builder_keeps_no_graph() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = discarding_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
    let _: [BlockNumber; 3] = written_graph(&mut builder);
    let graph = flow_of(&elements);
    usize::from(
        graph.blocks.is_empty()
            && graph.edges.is_empty()
            && graph.steps.is_empty()
            && graph.members.is_empty(),
    )
}

/// A file's graph, then a second file's table built with one block: 1 when the second file sees its block alone, numbered 0.
fn the_next_file_sees_none_of_the_last_files_graph() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
    let _: [BlockNumber; 3] = written_graph(&mut builder);
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"y"));
    let number = add_flow_block(
        &mut builder,
        block(Flag::Lowered, Flag::Raised, Flag::Lowered),
    );
    let graph = flow_of(&elements);
    usize::from(
        number == BlockNumber(0)
            && graph.blocks == [block(Flag::Lowered, Flag::Raised, Flag::Lowered)]
            && graph.edges.is_empty()
            && graph.steps.is_empty()
            && graph.members.is_empty(),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("blocks_are_numbered_from_zero_in_the_order_given"),
        word: Word::Succeeds,
        input: || blocks_are_numbered_from_zero_in_the_order_given,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_discarding_builder_keeps_no_graph"),
        word: Word::Succeeds,
        input: || a_discarding_builder_keeps_no_graph,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_next_file_sees_none_of_the_last_files_graph"),
        word: Word::Succeeds,
        input: || the_next_file_sees_none_of_the_last_files_graph,
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
