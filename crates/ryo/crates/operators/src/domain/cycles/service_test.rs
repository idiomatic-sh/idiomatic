//! A component of more than one node and a self edge are cycles; a chain is not. The search asks for each edge once and for one
//! edge past the last of each node: `nodes + edges` asks, one pass of the graph, however many cycles the graph holds.
use super::service::cycles;
use super::types::{CycleDriver, CycleScratch, EdgePlace, NodeCount, NodeNumber};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
/// Compressed adjacency: the edges leaving node `n` are `leaving[starts[n]..starts[n + 1]]`, with a count of the edges asked for.
struct Graph {
    values: Vec<Cell>,
    starts: std::borrow::Cow<'static, [usize]>,
    leaving: std::borrow::Cow<'static, [usize]>,
    asked: std::cell::Cell<usize>,
}
impl CycleDriver for Graph {
    fn nodes(&self) -> NodeCount {
        NodeCount(self.values.len())
    }
    fn value(&self, node: NodeNumber) -> Option<Cell> {
        self.values.get(node.0).copied()
    }
    fn edge(&self, node: NodeNumber, place: EdgePlace) -> Option<NodeNumber> {
        self.asked.set(self.asked.get() + 1);
        let at = self.starts.get(node.0)?.checked_add(place.0)?;
        (at < *self.starts.get(node.0 + 1)?)
            .then(|| self.leaving.get(at).copied().map(NodeNumber))
            .flatten()
    }
}
/// The value of every node of a ring: each is on a cycle, so the answer holds it once per node.
const RING_VALUE: i64 = 7;
/// A graph a row gives the search: listed as compressed adjacency, or a ring of `n` nodes where node `i` leads to `i + 1` and to
/// `i + 2`, both around the ring.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// Values, starts and leaving edges as written.
    Listed(&'static [i64], &'static [usize], &'static [usize]),
    /// A ring with a chord past each next node: one component of every node, `2n` edges.
    ChordedRing(usize),
}
type Input = Shape;
/// The values on a cycle, ascending, and how many edges the search asked the graph for.
type Value = (std::borrow::Cow<'static, [i64]>, usize);
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_two_node_loop_marks_both"),
        word: Word::Succeeds,
        input: || Shape::Listed(&[5, 3], &[0, 1, 2], &[1, 0]),
        expect: Expect::Value((std::borrow::Cow::Borrowed(&[3, 5]), 4)),
    },
    Case {
        name: CaseName("a_self_edge_marks_its_node"),
        word: Word::Succeeds,
        input: || Shape::Listed(&[1, 2], &[0, 1, 1], &[0]),
        expect: Expect::Value((std::borrow::Cow::Borrowed(&[1]), 3)),
    },
    Case {
        name: CaseName("a_chain_marks_nothing"),
        word: Word::Succeeds,
        input: || Shape::Listed(&[1, 2, 3], &[0, 1, 2, 2], &[1, 2]),
        expect: Expect::Value((std::borrow::Cow::Borrowed(&[]), 5)),
    },
    Case {
        name: CaseName("a_graph_of_many_cycles_is_searched_in_one_pass_of_its_edges"),
        word: Word::Succeeds,
        input: || Shape::ChordedRing(4),
        expect: Expect::Value((std::borrow::Cow::Borrowed(&[RING_VALUE; 4]), 12)),
    },
    Case {
        name: CaseName("a_thousand_node_graph_asks_for_each_edge_once_and_one_past_each_node"),
        word: Word::Succeeds,
        input: || Shape::ChordedRing(1000),
        expect: Expect::Value((std::borrow::Cow::Borrowed(&[RING_VALUE; 1000]), 3000)),
    },
];
fn subject(shape: Input) -> Result<Value, Failure> {
    let graph = match shape {
        Shape::Listed(values, starts, leaving) => Graph {
            values: values.iter().map(|each| Cell::Number(Integer(*each))).collect(),
            starts: std::borrow::Cow::Borrowed(starts),
            leaving: std::borrow::Cow::Borrowed(leaving),
            asked: std::cell::Cell::new(0),
        },
        Shape::ChordedRing(nodes) => Graph {
            values: vec![Cell::Number(Integer(RING_VALUE)); nodes],
            starts: std::borrow::Cow::Owned((0..=nodes).map(|node| 2 * node).collect()),
            leaving: std::borrow::Cow::Owned(
                (0..nodes)
                    .flat_map(|node| [(node + 1) % nodes, (node + 2) % nodes])
                    .collect(),
            ),
            asked: std::cell::Cell::new(0),
        },
    };
    let mut scratch = CycleScratch::default();
    cycles(&graph, &mut scratch);
    Ok((
        std::borrow::Cow::Owned(
            scratch
                .ordered
                .iter()
                .map(|value| match value {
                    Cell::Number(Integer(number)) => *number,
                    _ => i64::MIN,
                })
                .collect(),
        ),
        graph.asked.get(),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
