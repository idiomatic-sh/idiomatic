//! The `probe` item's cases, and the collection's shared cases when it has any: which lowerings run the kernel.
use super::item::ProbeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use crate::domain::set_rows::types::{SetKind, SetLowering};

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
            SetLowering::Probe(probe) => Lowering::Probe {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                probe,
            },
            SetLowering::Distinct => Lowering::Unlowered,
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_union_does_not_run_it"),
        word: Word::Succeeds,
        input: || Lowering::Distinct {
            sources: [
                RowSource::Node(PlanNodeId(0)),
                RowSource::Node(PlanNodeId(1)),
            ],
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
    Ok(ProbeItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
