//! The plan's growth check refuses a live node whose rows grow past its inputs' unless the query declared every pair, and
//! judges no node a rewrite left unread.
use super::errors::GrowthError;
use super::service::checked;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::Degree;
use crate::domain::physical_plan::types::lowering::value::{
    BandJoinNode, BoundScan, FilterNode, Lowering, ScanBoundNode,
};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{
    EngineUses, NodeFlow, NodeReaders, PlanNodeId, RowSource,
};
use ryo_values::domain::band_joins::types::{BandAnswer, BandEdge, BandTest};
use ryo_values::domain::plan_parts::types::{ColumnIndex, HostedNumber, JoinColumn, KeyFan, Side};

/// Two scans, a band join of them answering this way with these keys, and a filter of it kept by the engine; the band join
/// read by the filter or by nothing.
#[derive(Debug)]
struct Given(BandAnswer, KeyFan, Readers);

/// Whether a node is read.
#[derive(Clone, Copy, Debug)]
enum Readers {
    /// The filter reads the band join.
    Read,
    /// Nothing reads the band join: a rewrite left it behind.
    Unread,
}

type Input = Given;
/// Each node's output rows degree, in id order.
type Value = std::borrow::Cow<'static, [Degree]>;
type Failure = GrowthError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_nearest_band_is_one_row_per_left_row"),
        word: Word::Succeeds,
        input: || Given(BandAnswer::Nearest, KeyFan::Shared, Readers::Read),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Degree(1),
            Degree(1),
            Degree(1),
            Degree(1),
        ])),
    },
    Case {
        name: CaseName("a_declared_band_and_the_filter_after_it_are_accepted"),
        word: Word::Succeeds,
        input: || Given(BandAnswer::Every, KeyFan::AllPairs, Readers::Read),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Degree(1),
            Degree(1),
            Degree(2),
            Degree(2),
        ])),
    },
    Case {
        name: CaseName("a_file_only_band_no_node_reads_is_not_judged"),
        word: Word::Succeeds,
        input: || Given(BandAnswer::Every, KeyFan::FileOnly, Readers::Unread),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Degree(1),
            Degree(1),
            Degree(2),
            Degree(1),
        ])),
    },
    Case {
        name: CaseName("a_file_only_band_over_two_relations_is_refused"),
        word: Word::Refuses,
        input: || Given(BandAnswer::Every, KeyFan::FileOnly, Readers::Read),
        expect: Expect::Error(GrowthError::Superlinear {
            node: PlanNodeId(2),
            kind: "BandJoin",
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

/// The plan a case gives.
fn plan(answer: BandAnswer, fan: KeyFan, readers: Readers) -> Vec<PhysicalNode> {
    let scan = || {
        Lowering::ScanBound(ScanBoundNode {
            scan: BoundScan::Hosted(HostedNumber(0)),
        })
    };
    let at = ColumnIndex(0);
    let band = crate::domain::equality_join::service::banded(
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

fn subject(Given(answer, fan, readers): Input) -> Result<Value, Failure> {
    Ok(checked(&plan(answer, fan, readers))?
        .iter()
        .map(|each| each.output.rows)
        .collect())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
