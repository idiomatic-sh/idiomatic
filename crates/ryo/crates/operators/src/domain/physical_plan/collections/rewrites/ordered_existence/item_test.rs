//! The `ordered_existence` item's cases: the shapes it rewrites and the ones it declines.
use super::item::OrderedExistenceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem as _;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;

/// The lowered nodes of a plan.
type Input = Vec<PhysicalNode>;
/// The nodes after the rewrite.
type Value = Vec<PhysicalNode>;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_plan_of_no_antijoin_is_unchanged"),
    word: Word::Succeeds,
    input: Vec::new,
    expect: Expect::Value(Vec::new()),
}];

/// Calls the part on one input; the author states it.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; a rewrite cannot fail"
)]
fn subject(mut nodes: Input) -> Result<Value, Failure> {
    OrderedExistenceItem.rewrite(&mut nodes, &mut Vec::new());
    Ok(nodes)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
