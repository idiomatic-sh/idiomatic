//! The validators' cases, on rows written by hand: a real empty module's flow graph holds, and a planted edge to a block the file
//! does not have, an edge numbered past its place among its source's edges, or a member for an element it does not have, is
//! refused.

use super::{empty_module_flow, flow_valid};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;

/// A scenario: what a row asks of the validators.
type Input = fn() -> bool;
/// Whether the validators accept the rows.
type Value = bool;
type Failure = ();

/// The element id of the module's root.
const ROOT: u64 = 0;
/// The symbol a hand-written edge or step kind is spelled with: the one word `word` accepts.
const KIND: u64 = 7;

/// An empty module's graph as a language writes it: its error exit (0) and entry (1), the edge from the entry to the exit, and the
/// root in the entry, with `edge` and `member` in place of that edge and that member.
fn module(edge: [u64; 5], member: [u64; 3]) -> Vec<(Relation, Vec<u64>)> {
    vec![
        (Relation::FlowBlock, vec![0, 1, 1, 0, 0]),
        (Relation::FlowBlock, vec![1, 1, 1, 0, 0]),
        (Relation::FlowEdge, edge.to_vec()),
        (Relation::FlowMember, member.to_vec()),
    ]
}

/// Whether `rows` pass the flow validator, the root being the file's one element, and an empty module's check of each relation.
fn accepted(rows: &[(Relation, Vec<u64>)]) -> bool {
    flow_valid(rows, |id| id == ROOT, |_, kind| kind == KIND)
        && [
            Relation::FlowBlock,
            Relation::FlowEdge,
            Relation::FlowMember,
        ]
        .into_iter()
        .all(|relation| empty_module_flow(rows, relation, ROOT))
}

/// The module's own graph.
fn a_real_empty_module_graph_holds() -> bool {
    accepted(&module([1, 0, KIND, 0, 0], [ROOT, 1, 0]))
}

/// The entry's edge planted to block 5, which the file does not have.
fn an_edge_to_a_block_the_file_lacks_is_refused() -> bool {
    accepted(&module([1, 5, KIND, 0, 0], [ROOT, 1, 0]))
}

/// The entry's one edge numbered 1, past its place among the entry's edges.
fn an_edge_numbered_past_its_place_is_refused() -> bool {
    accepted(&module([1, 0, KIND, 1, 0], [ROOT, 1, 0]))
}

/// The member planted for element 9, which the file does not have.
fn a_member_for_an_element_the_file_lacks_is_refused() -> bool {
    accepted(&module([1, 0, KIND, 0, 0], [9, 1, 0]))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_real_empty_module_graph_holds"),
        word: Word::Succeeds,
        input: || a_real_empty_module_graph_holds,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_edge_to_a_block_the_file_lacks_is_refused"),
        word: Word::Succeeds,
        input: || an_edge_to_a_block_the_file_lacks_is_refused,
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("an_edge_numbered_past_its_place_is_refused"),
        word: Word::Succeeds,
        input: || an_edge_numbered_past_its_place_is_refused,
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_member_for_an_element_the_file_lacks_is_refused"),
        word: Word::Succeeds,
        input: || a_member_for_an_element_the_file_lacks_is_refused,
        expect: Expect::Value(false),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
