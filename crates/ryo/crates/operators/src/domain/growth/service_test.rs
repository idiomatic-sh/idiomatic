//! The one algebra composes a node's bound from its declaration and its inputs' bounds.
use super::errors::GrowthError;
use super::service::judged;
use super::types::{Bound, Degree, Fan, Growth, GrowthKind, GrowthNode, NodeBound};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A node of this growth over two relations' rows.
type Input = Growth;
/// Its bound.
type Value = NodeBound;
/// The algebra refuses nothing.
type Failure = GrowthError;

/// `rows^a · depth^b · repeats^c`.
const fn bound(rows: u8, depth: u8, repeats: u8) -> Bound {
    Bound {
        rows: Degree(rows),
        depth: Degree(depth),
        repeats: Degree(repeats),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_writes_at_most_its_input"),
        word: Word::Succeeds,
        input: || Growth::AtMostInput,
        expect: Expect::Value(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        }),
    },
    Case {
        name: CaseName("a_text_map_writes_at_most_the_partition_text"),
        word: Word::Succeeds,
        input: || Growth::PerRowText,
        expect: Expect::Value(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        }),
    },
    Case {
        name: CaseName("an_ordered_match_writes_one_row_per_left_row"),
        word: Word::Succeeds,
        input: || Growth::OnePerLeftRow,
        expect: Expect::Value(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        }),
    },
    Case {
        name: CaseName("a_unique_probe_writes_one_row_per_left_row"),
        word: Word::Succeeds,
        input: || Growth::Keyed(Fan::PerLeft),
        expect: Expect::Value(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        }),
    },
    Case {
        name: CaseName("a_shared_probe_writes_the_repeats_of_a_key"),
        word: Word::Succeeds,
        input: || Growth::Keyed(Fan::Repeats),
        expect: Expect::Value(NodeBound {
            output: bound(1, 0, 1),
            work: bound(1, 0, 1),
        }),
    },
    Case {
        name: CaseName("a_file_only_probe_writing_the_product_of_its_sides_is_refused"),
        word: Word::Refuses,
        input: || Growth::Keyed(Fan::FileProduct),
        expect: Expect::Error(GrowthError::Superlinear {
            node: GrowthNode(0),
            kind: GrowthKind("Probe"),
            degree: Degree(2),
        }),
    },
    Case {
        name: CaseName("a_declared_band_writes_every_pair"),
        word: Word::Succeeds,
        input: || Growth::Banded(Fan::DeclaredPairs),
        expect: Expect::Value(NodeBound {
            output: bound(2, 0, 0),
            work: bound(2, 0, 0),
        }),
    },
    Case {
        name: CaseName("a_walk_writes_its_starts_times_the_depth"),
        word: Word::Succeeds,
        input: || Growth::PerStartWalk,
        expect: Expect::Value(NodeBound {
            output: bound(1, 1, 0),
            work: bound(1, 1, 0),
        }),
    },
    Case {
        name: CaseName("a_group_writes_at_most_its_input"),
        word: Word::Succeeds,
        input: || Growth::GroupOutput,
        expect: Expect::Value(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        }),
    },
];

fn subject(growth: Input) -> Result<Value, Failure> {
    judged(
        growth,
        &[bound(1, 0, 0), bound(1, 0, 0)],
        GrowthNode(0),
        GrowthKind("Probe"),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
