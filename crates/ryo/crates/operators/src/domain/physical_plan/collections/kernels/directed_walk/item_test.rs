//! The `directed_walk` item's cases: which lowerings run the kernel.
use super::item::DirectedWalkItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::axis::types::AxisShape;
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{AxisNode, DistinctNode, Lowering};
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::{Direction, JoinColumn, Side};

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
    Ok(DirectedWalkItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for AxisNode {
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
                starts: RowSource::Node(PlanNodeId(0)),
                edges: RowSource::Node(PlanNodeId(1)),
                domain: RowSource::Node(PlanNodeId(2)),
                axis: crate::domain::axis::service::lower(
                    AxisShape {
                        direction: Direction::Descendant,
                        id: columns.id,
                        parent: columns.parent,
                        child: columns.child,
                        domain_id: columns.id,
                        output: &output,
                    },
                    &[],
                    &[],
                    &[],
                ),
            },
            roles: vec![InputRole::Rows, InputRole::Edges, InputRole::Domain],
        }]
    }
}
