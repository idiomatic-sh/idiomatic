//! The `reach` item's cases: which lowerings run the kernel.
use super::item::ReachItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::equality_join::types::JoinShape;
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{
    DistinctNode, Lowering, ReachNode, SeededReachNode,
};
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, ReachLookup, Side};

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
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
}];

/// Calls the part on one input; the author states it.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; whether a lowering runs a kernel cannot fail"
)]
fn subject(lowering: Input) -> Result<Value, Failure> {
    Ok(ReachItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for ReachNode {
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        vec![Witnessed {
            node: Self {
                edges: RowSource::Node(PlanNodeId(0)),
                graph: crate::domain::closure::service::lower(&[], columns.parent, columns.child),
            },
            roles: vec![InputRole::Edges],
        }]
    }
}

impl Witnesses for SeededReachNode {
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        // The reached rows as the join reads them: the source at 0, the entity reached at 1.
        let reach = ReachLookup {
            source: columns.parent,
            destination: columns.child,
            keys: Vec::new(),
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
        };
        let output = [
            JoinColumn {
                side: Side::Left,
                column: columns.id,
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
        ];
        vec![Witnessed {
            node: Self {
                left: RowSource::Node(PlanNodeId(0)),
                edges: RowSource::Node(PlanNodeId(1)),
                graph: crate::domain::closure::service::lower(&[], columns.parent, columns.child),
                join: crate::domain::equality_join::service::seeded(
                    JoinShape {
                        left_on: &[columns.id],
                        right_on: &[ColumnIndex(0)],
                        output: &output,
                        view: &[],
                        reach: Some(&reach),
                        fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
                    },
                    &[],
                    &[],
                ),
            },
            roles: vec![InputRole::Rows, InputRole::Edges],
        }]
    }
}
