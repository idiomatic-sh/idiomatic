//! Candidates answer their tree nodes in the order given.
use super::value::Candidates;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::NodeId;

type Input = Vec<NodeId>;
/// How many nodes, and the first.
type Value = (usize, Option<NodeId>);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("candidates_answer_their_nodes_in_order"),
        word: Word::Succeeds,
        input: || vec![NodeId(3), NodeId(5)],
        expect: Expect::Value((2, Some(NodeId(3)))),
    },
];

fn subject(nodes: Input) -> Result<Value, Failure> {
    let nodes = Candidates(&nodes).nodes();
    Ok((nodes.len(), nodes.first().copied()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
