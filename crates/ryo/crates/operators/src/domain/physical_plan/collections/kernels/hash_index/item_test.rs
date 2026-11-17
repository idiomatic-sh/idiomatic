//! The `hash_index` item's cases, and the collection's shared cases when it has any: which lowerings run the kernel.
use super::item::HashIndexItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_group_runs_it"),
        word: Word::Succeeds,
        input: || Lowering::Group {
            source: RowSource::Node(PlanNodeId(0)),
            group: crate::domain::aggregate::service::lower(
                &[],
                crate::domain::aggregate::types::GroupMeasure::Rows,
            ),
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_projection_does_not_run_it"),
        word: Word::Succeeds,
        input: || Lowering::Project {
            source: RowSource::Node(PlanNodeId(0)),
            project: crate::domain::project::service::lower(&[], RepeatedRows::Kept),
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
    Ok(HashIndexItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
