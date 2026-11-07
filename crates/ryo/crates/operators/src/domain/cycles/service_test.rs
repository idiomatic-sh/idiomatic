//! A component of more than one node and a self edge are cycles; a chain is not.
use super::service::cycles;
use super::types::{CycleDriver, CycleScratch, EdgePlace, NodeCount, NodeNumber};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
/// Compressed adjacency: the edges leaving node `n` are `leaving[starts[n]..starts[n + 1]]`.
struct Graph {
    values: Vec<Cell>,
    starts: &'static [usize],
    leaving: &'static [usize],
}
impl CycleDriver for Graph {
    fn nodes(&self) -> NodeCount {
        NodeCount(self.values.len())
    }
    fn value(&self, node: NodeNumber) -> Option<Cell> {
        self.values.get(node.0).copied()
    }
    fn edge(&self, node: NodeNumber, place: EdgePlace) -> Option<NodeNumber> {
        let at = self.starts.get(node.0)?.checked_add(place.0)?;
        (at < *self.starts.get(node.0 + 1)?)
            .then(|| self.leaving.get(at).copied().map(NodeNumber))
            .flatten()
    }
}
type Input = (&'static [i64], &'static [usize], &'static [usize]);
type Value = std::borrow::Cow<'static, [i64]>;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_two_node_loop_marks_both"),
        word: Word::Succeeds,
        input: || (&[5, 3], &[0, 1, 2], &[1, 0]),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[3, 5])),
    },
    Case {
        name: CaseName("a_self_edge_marks_its_node"),
        word: Word::Succeeds,
        input: || (&[1, 2], &[0, 1, 1], &[0]),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("a_chain_marks_nothing"),
        word: Word::Succeeds,
        input: || (&[1, 2, 3], &[0, 1, 2, 2], &[1, 2]),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
];
fn subject((values, starts, leaving): Input) -> Result<Value, Failure> {
    let graph = Graph {
        values: values.iter().map(|each| Cell::Number(Integer(*each))).collect(),
        starts,
        leaving,
    };
    let mut scratch = CycleScratch::default();
    cycles(&graph, &mut scratch);
    Ok(std::borrow::Cow::Owned(
        scratch
            .ordered
            .iter()
            .map(|value| match value {
                Cell::Number(Integer(number)) => *number,
                _ => i64::MIN,
            })
            .collect(),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
