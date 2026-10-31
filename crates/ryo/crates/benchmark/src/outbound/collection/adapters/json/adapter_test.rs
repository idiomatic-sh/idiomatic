//! The production JSON codec satisfies every protocol observation.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("JSON satisfies collector contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    crate::outbound::collection::fixtures::contract::check(&super::adapter::CollectionJsonAdapter);
    crate::outbound::collection::fixtures::contract::check(
        &crate::outbound::collection::factory::json(),
    );
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
