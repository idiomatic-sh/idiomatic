//! The `lookup` item's cases, and the collection's shared cases when it has any: which lowerings run the kernel.
use super::item::LookupItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::{WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{FilterNode, LookupNode, Lowering};
use crate::domain::physical_plan::types::{IndexPlace, RowSource};
use ryo_relations::domain::relations::types::Relation;

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
        input: || {
            Lowering::Lookup(LookupNode {
                index: IndexPlace(0),
                // The text the generic rows hold, so the lookup answers rows.
                key: ryo_language::domain::symbols::service::hash_of(
                    ryo_language::domain::symbols::types::ByteSlice(
                        crate::domain::growth::types::WITNESS_TEXT.as_bytes(),
                    ),
                ),
                select: crate::domain::select::service::lower(&[], &[], &[]),
            })
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_filter_does_not_run_it"),
        word: Word::Succeeds,
        input: || {
            Lowering::Filter(FilterNode {
                source: RowSource::Facts(Relation::Defines),
                select: crate::domain::select::service::lower(&[], &[], &[]),
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
    Ok(LookupItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for LookupNode {
    fn witnesses(_: WitnessColumns) -> Vec<Witnessed<Self>> {
        // A lookup reads its index's relation, not a node: it names no input.
        vec![Witnessed {
            node: Self {
                index: IndexPlace(0),
                // The text the generic rows hold, so the lookup answers rows.
                key: ryo_language::domain::symbols::service::hash_of(
                    ryo_language::domain::symbols::types::ByteSlice(
                        crate::domain::growth::types::WITNESS_TEXT.as_bytes(),
                    ),
                ),
                select: crate::domain::select::service::lower(&[], &[], &[]),
            },
            roles: Vec::new(),
        }]
    }
}
