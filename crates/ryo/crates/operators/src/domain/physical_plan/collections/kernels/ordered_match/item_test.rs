//! The `ordered_match` item's cases: which lowerings run the kernel.
use super::item::OrderedMatchItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::{Lowering, PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::ColumnIndex;

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
        input: || Lowering::OrderedMatch {
            left: RowSource::Node(PlanNodeId(0)),
            right: RowSource::Node(PlanNodeId(1)),
            ordered: crate::domain::ordered_match::service::lower(
                &[],
                &[],
                ColumnIndex(0),
                ColumnIndex(0),
                &[],
            ),
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_projection_does_not_run_it"),
        word: Word::Succeeds,
        input: || Lowering::Project {
            source: RowSource::Node(PlanNodeId(0)),
            project: crate::domain::project::service::lower(
                &[ColumnIndex(0)],
                ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
            ),
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
