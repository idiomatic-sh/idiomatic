//! The in-process implementation executes the complete shared contract: every invocation the contract makes is a child's.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::fixtures::contract::check;
use crate::outbound::execution::types::InProcessWorker;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
type Input = ();
type Value = ();
type Failure = crate::outbound::execution::fixtures::contract::LaunchFailure;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("in-process implementation answers the shared contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::execution::factory::in_process(
        Box::new(crate::outbound::execution::factory::process()),
        InProcessWorker {
            run: Box::new(|_request, _response| {
                Err(ReportsError::Invalid("the contract runs no worker".into()))
            }),
            compile: Box::new(|_request| Ok(None)),
        },
    ));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
