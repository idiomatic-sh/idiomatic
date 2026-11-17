//! The `components` item's cases: which lowerings run the kernel.
use super::item::ComponentsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{
    ComponentsNode, DistinctNode, Lowering,
};
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};

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
    Ok(ComponentsItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for ComponentsNode {
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        vec![Witnessed {
            node: Self {
                edges: RowSource::Node(PlanNodeId(0)),
                cycles: crate::domain::cycles::service::lower(columns.parent, columns.child),
            },
            roles: vec![InputRole::Edges],
        }]
    }
}
