//! The `probe_right` item's cases: which lowerings run the kernel, a probe's and a seeded closure's join.
use super::item::ProbeRightItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::equality_join::types::{
    JoinShape, OrderedProbe, PartitionExtremum, ProbeMode, ProbeParts,
};
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{DistinctNode, Lowering, ProbeNode};
use crate::domain::physical_plan::types::{PlanNodeId, ProbeIndex, RowSource};
use crate::domain::set_rows::types::{SetKind, SetLowering};
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_antijoin_runs_it"),
        word: Word::Succeeds,
        input: || match crate::domain::set_rows::service::lower(SetKind::Antijoin, &[], &[]) {
            SetLowering::Probe(probe) => Lowering::Probe(ProbeNode {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                probe,
                index: ProbeIndex::Own,
            }),
            SetLowering::Distinct => Lowering::Distinct(DistinctNode {
                sources: [
                    RowSource::Node(PlanNodeId(0)),
                    RowSource::Node(PlanNodeId(1)),
                ],
            }),
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_union_does_not_run_it"),
        word: Word::Succeeds,
        input: || {
            Lowering::Distinct(DistinctNode {
                sources: [
                    RowSource::Node(PlanNodeId(0)),
                    RowSource::Node(PlanNodeId(1)),
                ],
            })
        },
        expect: Expect::Value(Reach::Skips),
    },
];

/// Calls the part on one input; the author states it.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; whether a lowering runs a kernel cannot fail"
)]
fn subject(lowering: Input) -> Result<Value, Failure> {
    Ok(ProbeRightItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for ProbeNode {
    /// One instance per mode: inner, semi (an intersection), anti (an antijoin) and ordered.
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        let on = [columns.key];
        let output = [
            JoinColumn {
                side: Side::Left,
                column: columns.id,
            },
            JoinColumn {
                side: Side::Right,
                column: columns.id,
            },
        ];
        let inner = crate::domain::equality_join::service::lower(
            JoinShape {
                left_on: &on,
                right_on: &on,
                output: &output,
                view: &[],
                reach: None,
                fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
            },
            &[],
            &[],
        );
        let set = |kind| match crate::domain::set_rows::service::lower(kind, &on, &on) {
            SetLowering::Probe(probe) => Some(probe),
            SetLowering::Distinct => None,
        };
        let ordered = set(SetKind::Antijoin).map(|probe| ProbeParts {
            mode: ProbeMode::Ordered(OrderedProbe {
                left_order: columns.order,
                right_order: columns.order,
                extremum: PartitionExtremum::Maximum,
            }),
            ..probe
        });
        [
            Some(inner),
            set(SetKind::Intersection),
            set(SetKind::Antijoin),
            ordered,
        ]
        .into_iter()
        .flatten()
        .map(|probe| Witnessed {
            node: Self {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                probe,
                index: ProbeIndex::Own,
            },
            roles: vec![InputRole::Rows, InputRole::Rows],
        })
        .collect()
    }
}
