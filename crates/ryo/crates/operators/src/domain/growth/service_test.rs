//! The one algebra composes a node's bound from its declaration and its inputs' bounds, and the one check refuses a live node
//! whose rows grow past its inputs' unless the query declared every pair.
use super::errors::GrowthError;
use super::service::{checked, composed};
use super::types::{Bound, Degree, Fan, Growth, NodeBound};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::lowering::value::{
    BandJoinNode, BoundScan, FilterNode, Lowering, ScanBoundNode,
};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{
    EngineUses, NodeFlow, NodeReaders, PlanNodeId, RowSource,
};
use ryo_values::domain::plan_parts::types::{
    BandAnswer, BandEdge, BandTest, ColumnIndex, HostedNumber, JoinColumn, KeyFan, Side,
};

/// What a case gives the part.
#[derive(Debug)]
enum Given {
    /// A node of this growth over two relations' rows.
    Composed(Growth),
    /// Two scans, a band join of them answering this way with these keys, and a filter of it kept by the engine; the band join
    /// read by the filter or by nothing.
    Plan(BandAnswer, KeyFan, Readers),
}

/// Whether a node is read.
#[derive(Clone, Copy, Debug)]
enum Readers {
    /// The filter reads the band join.
    Read,
    /// Nothing reads the band join: a rewrite left it behind.
    Unread,
}

/// What the part answered.
#[derive(Debug, PartialEq)]
enum Answer {
    /// A node's bound.
    Bound(NodeBound),
    /// Each node's output rows degree, in id order.
    Degrees(std::borrow::Cow<'static, [Degree]>),
}

type Input = Given;
type Value = Answer;
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
        input: || Given::Composed(Growth::AtMostInput),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        })),
    },
    Case {
        name: CaseName("a_text_map_writes_at_most_the_partition_text"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::PerRowText),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        })),
    },
    Case {
        name: CaseName("an_ordered_match_writes_one_row_per_left_row"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::OnePerLeftRow),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        })),
    },
    Case {
        name: CaseName("a_unique_probe_writes_one_row_per_left_row"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::Keyed(Fan::PerLeft)),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        })),
    },
    Case {
        name: CaseName("a_shared_probe_writes_the_repeats_of_a_key"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::Keyed(Fan::Repeats)),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(1, 0, 1),
            work: bound(1, 0, 1),
        })),
    },
    Case {
        name: CaseName("a_file_only_probe_writes_the_product_of_its_sides"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::Keyed(Fan::FileProduct)),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(2, 0, 0),
            work: bound(2, 0, 0),
        })),
    },
    Case {
        name: CaseName("a_declared_band_writes_every_pair"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::Banded(Fan::DeclaredPairs)),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(2, 0, 0),
            work: bound(2, 0, 0),
        })),
    },
    Case {
        name: CaseName("a_walk_writes_its_starts_times_the_depth"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::PerStartWalk),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(1, 1, 0),
            work: bound(1, 1, 0),
        })),
    },
    Case {
        name: CaseName("a_group_writes_at_most_its_input"),
        word: Word::Succeeds,
        input: || Given::Composed(Growth::GroupOutput),
        expect: Expect::Value(Answer::Bound(NodeBound {
            output: bound(1, 0, 0),
            work: bound(1, 0, 0),
        })),
    },
    Case {
        name: CaseName("a_nearest_band_is_one_row_per_left_row"),
        word: Word::Succeeds,
        input: || Given::Plan(BandAnswer::Nearest, KeyFan::Shared, Readers::Read),
        expect: Expect::Value(Answer::Degrees(std::borrow::Cow::Borrowed(&[
            Degree(1),
            Degree(1),
            Degree(1),
            Degree(1),
        ]))),
    },
    Case {
        name: CaseName("a_declared_band_and_the_filter_after_it_are_accepted"),
        word: Word::Succeeds,
        input: || Given::Plan(BandAnswer::Every, KeyFan::AllPairs, Readers::Read),
        expect: Expect::Value(Answer::Degrees(std::borrow::Cow::Borrowed(&[
            Degree(1),
            Degree(1),
            Degree(2),
            Degree(2),
        ]))),
    },
    Case {
        name: CaseName("a_file_only_band_no_node_reads_is_not_judged"),
        word: Word::Succeeds,
        input: || Given::Plan(BandAnswer::Every, KeyFan::FileOnly, Readers::Unread),
        expect: Expect::Value(Answer::Degrees(std::borrow::Cow::Borrowed(&[
            Degree(1),
            Degree(1),
            Degree(2),
            Degree(1),
        ]))),
    },
    Case {
        name: CaseName("a_file_only_band_over_two_relations_is_refused"),
        word: Word::Refuses,
        input: || Given::Plan(BandAnswer::Every, KeyFan::FileOnly, Readers::Read),
        expect: Expect::Error(GrowthError::Superlinear {
            node: PlanNodeId(2),
            growth: Growth::Banded(Fan::FileProduct),
            degree: Degree(2),
        }),
    },
];

/// A node of `lowering` read by `readers` nodes and kept by `engine` uses.
const fn node(lowering: Lowering, readers: u32, engine: u32) -> PhysicalNode {
    PhysicalNode {
        lowering,
        readers: NodeReaders(readers),
        engine: EngineUses(engine),
        flow: NodeFlow::Held,
    }
}

/// The plan of `Given::Plan`.
fn plan(answer: BandAnswer, fan: KeyFan, readers: Readers) -> Vec<PhysicalNode> {
    let scan = || {
        Lowering::ScanBound(ScanBoundNode {
            scan: BoundScan::Hosted(HostedNumber(0)),
        })
    };
    let at = ColumnIndex(0);
    let band = crate::domain::band_join::service::lower(
        [&[at], &[at]],
        &[BandEdge {
            left: at,
            test: BandTest::Below,
            right: at,
        }],
        answer,
        &[JoinColumn {
            side: Side::Left,
            column: at,
        }],
        &[],
        fan,
    );
    let mut nodes = vec![node(scan(), 1, 0), node(scan(), 1, 0)];
    if let Some(band) = band {
        nodes.push(node(
            Lowering::BandJoin(BandJoinNode {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                band,
            }),
            match readers {
                Readers::Read => 1,
                Readers::Unread => 0,
            },
            0,
        ));
    }
    let read = match readers {
        Readers::Read => RowSource::Node(PlanNodeId(2)),
        Readers::Unread => RowSource::Node(PlanNodeId(0)),
    };
    nodes.push(node(
        Lowering::Filter(FilterNode {
            source: read,
            select: crate::domain::select::service::lower(&[], &[], &[]),
        }),
        0,
        1,
    ));
    nodes
}

fn subject(given: Input) -> Result<Value, Failure> {
    let two = [bound(1, 0, 0), bound(1, 0, 0)];
    match given {
        Given::Composed(growth) => Ok(Answer::Bound(composed(growth, &two))),
        Given::Plan(answer, fan, readers) => Ok(Answer::Degrees(
            checked(&plan(answer, fan, readers))?
                .iter()
                .map(|each| each.output.rows)
                .collect(),
        )),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
