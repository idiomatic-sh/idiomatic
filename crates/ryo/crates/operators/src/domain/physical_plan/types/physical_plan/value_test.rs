//! A plan that lowered every node records none as unserved.
use super::value::PhysicalPlan;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// How many nodes the plan could not lower yet.
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_plan_with_every_node_lowered_has_none_unserved"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        Ok(PhysicalPlan::default().unserved().len())
    });
}
