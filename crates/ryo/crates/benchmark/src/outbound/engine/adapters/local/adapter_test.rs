//! The concrete local implementation executes the complete shared contract.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("concrete implementation answers the shared contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::engine::factory::local(
        ryo::linter::Languages::default(),
        None,
    ));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
