//! The concrete ryo implementation executes the complete shared contract.
use super::adapter::LinterRyoAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::fixtures::contract::check;
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
    check(&LinterRyoAdapter {
        executable: "fixture".into(),
        rules: "rules".into(),
    });
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
