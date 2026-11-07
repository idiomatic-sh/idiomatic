//! The concrete stream destination answers the publication contract.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("BMF destination honors publication contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::evidence::factory::bmf(
        Box::new(std::io::sink()),
        Box::new(std::io::sink()),
    ));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
