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
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&LinterRyoAdapter {
        executable: "fixture".into(),
        rules: "rules".into(),
        query_modules: std::collections::BTreeMap::default(),
        parser_configuration: None,
    });
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
