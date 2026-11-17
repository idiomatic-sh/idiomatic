//! The `ordered_match` item's cases: which lowerings run the kernel.
use super::item::OrderedMatchItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{
    Lowering, OrderedMatchNode, ProjectNode,
};
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_ordered_match_runs_it"),
        word: Word::Succeeds,
        input: || {
            Lowering::OrderedMatch(OrderedMatchNode {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                ordered: crate::domain::ordered_match::service::lower(
                    &[],
                    &[],
                    ColumnIndex(0),
                    ColumnIndex(0),
                    &[],
                ),
            })
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_projection_does_not_run_it"),
        word: Word::Succeeds,
        input: || {
            Lowering::Project(ProjectNode {
                source: RowSource::Node(PlanNodeId(0)),
                project: crate::domain::project::service::lower(
                    &[ColumnIndex(0)],
                    ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
                ),
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
    Ok(OrderedMatchItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for OrderedMatchNode {
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
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
        vec![Witnessed {
            node: Self {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                ordered: crate::domain::ordered_match::service::lower(
                    &[columns.key],
                    &[columns.key],
                    columns.order,
                    columns.order,
                    &output,
                ),
            },
            roles: vec![InputRole::Rows, InputRole::Rows],
        }]
    }
}
