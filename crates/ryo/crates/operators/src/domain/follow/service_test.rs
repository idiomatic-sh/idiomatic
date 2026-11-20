//! The `follow` service's cases: fixed points, dead ends, cycles, several targets and depth.
use super::service::fixed_points;
use super::types::{Entity, GroupPointers, Pointer};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// One key group's edges, as a row writes them.
#[derive(Debug)]
enum Edges {
    /// These edges, each a source and a destination, sorted by source.
    Written(&'static [(u32, u32)]),
    /// A chain of this many edges from entity 0, each to the next entity, ending in an entity that points to itself.
    Chain(u32),
}
type Input = Edges;
/// How many rows are answered, and the first of them, each a source and its fixed point.
type Value = (usize, std::borrow::Cow<'static, [(u32, u32)]>);
type Failure = std::convert::Infallible;

/// One more edge than the depth bound the pointer ladder had: 2^20.
const DEEP: u32 = 1_048_577;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_source_answers_its_fixed_point_and_a_self_pointer_answers_itself"),
        word: Word::Succeeds,
        input: || Edges::Written(&[(1, 2), (2, 3), (3, 3)]),
        expect: Expect::Value((3, std::borrow::Cow::Borrowed(&[(1, 3), (2, 3), (3, 3)]))),
    },
    Case {
        name: CaseName("a_walk_that_runs_out_of_edges_has_no_row"),
        word: Word::Succeeds,
        input: || Edges::Written(&[(1, 2), (2, 9), (5, 5)]),
        expect: Expect::Value((1, std::borrow::Cow::Borrowed(&[(5, 5)]))),
    },
    Case {
        name: CaseName("a_cycle_longer_than_one_has_no_row"),
        word: Word::Succeeds,
        input: || Edges::Written(&[(1, 2), (2, 1), (7, 1)]),
        expect: Expect::Value((0, std::borrow::Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("every_reachable_fixed_point_is_answered_when_a_source_has_several_targets"),
        word: Word::Succeeds,
        input: || Edges::Written(&[(1, 2), (1, 3), (2, 2), (3, 3)]),
        expect: Expect::Value((
            4,
            std::borrow::Cow::Borrowed(&[(1, 2), (1, 3), (2, 2), (3, 3)]),
        )),
    },
    Case {
        name: CaseName("a_cycle_that_also_reaches_a_fixed_point_answers_it"),
        word: Word::Succeeds,
        input: || Edges::Written(&[(1, 2), (2, 1), (2, 4), (4, 4)]),
        expect: Expect::Value((3, std::borrow::Cow::Borrowed(&[(1, 4), (2, 4), (4, 4)]))),
    },
    Case {
        name: CaseName("a_repeated_edge_answers_its_pair_once"),
        word: Word::Succeeds,
        input: || Edges::Written(&[(1, 3), (1, 3), (3, 3), (3, 3)]),
        expect: Expect::Value((2, std::borrow::Cow::Borrowed(&[(1, 3), (3, 3)]))),
    },
    Case {
        name: CaseName("no_edges_answer_no_rows"),
        word: Word::Succeeds,
        input: || Edges::Written(&[]),
        expect: Expect::Value((0, std::borrow::Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("a_chain_longer_than_2_20_edges_reaches_its_fixed_point"),
        word: Word::Succeeds,
        input: || Edges::Chain(DEEP),
        expect: Expect::Value((
            1_048_578,
            std::borrow::Cow::Borrowed(&[
                (0, 1_048_577),
                (1, 1_048_577),
                (2, 1_048_577),
                (3, 1_048_577),
            ]),
        )),
    },
];

fn subject(edges: Input) -> Result<Value, Failure> {
    let pointer = |(source, destination): (u32, u32)| Pointer {
        source: Entity(source),
        destination: Entity(destination),
    };
    let group: Vec<Pointer> = match edges {
        Edges::Written(written) => written.iter().copied().map(pointer).collect(),
        Edges::Chain(length) => (0..length)
            .map(|at| (at, at.saturating_add(1)))
            .chain([(length, length)])
            .map(pointer)
            .collect(),
    };
    let mut out = Vec::new();
    fixed_points(GroupPointers(&group), &mut out);
    let first: Vec<(u32, u32)> = out
        .iter()
        .take(4)
        .map(|pair| (pair.source.0, pair.destination.0))
        .collect();
    Ok((out.len(), first.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
