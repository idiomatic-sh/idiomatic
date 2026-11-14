//! The `filter` item's cases, and the collection's shared cases when it has any: which lowerings run the kernel.
use super::item::FilterItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::{IndexPlace, Lowering, PlanNodeId, RowSource};
use ryo_relations::domain::relations::types::TextHash;

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_lookup_runs_it"),
        word: Word::Succeeds,
        input: || Lowering::Lookup {
            index: IndexPlace(0),
            key: TextHash(0),
            select: crate::domain::select::service::lower(&[], &[], &[]),
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("an_extension_does_not_run_it"),
        word: Word::Succeeds,
        input: || Lowering::Extend {
            source: RowSource::Node(PlanNodeId(0)),
            extension: crate::domain::append::service::lower(&[], &[]),
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
    Ok(FilterItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
