//! The `scan_roots` item's cases: which lowerings run the kernel.
use super::item::ScanRootsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::{RowSource, lowering::value::Lowering};
use ryo_relations::domain::relations::types::Relation;

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_of_the_tree_runs_it"),
        word: Word::Succeeds,
        input: || Lowering::Filter {
            source: RowSource::Tree(Relation::SyntaxTree),
            select: crate::domain::select::service::lower(&[], &[], &[]),
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_filter_of_a_relation_does_not_run_it"),
        word: Word::Succeeds,
        input: || Lowering::Filter {
            source: RowSource::Facts(Relation::Defines),
            select: crate::domain::select::service::lower(&[], &[], &[]),
        },
        expect: Expect::Value(Reach::Skips),
    },
];

fn subject(lowering: Lowering) -> Result<Value, Failure> {
    Ok(ScanRootsItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
