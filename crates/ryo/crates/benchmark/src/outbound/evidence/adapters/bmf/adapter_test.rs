//! The concrete stream destination answers the publication contract.
use super::{adapter::EvidenceBmfAdapter, connection::EvidenceBmfConnection};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::fixtures::contract::check;
use std::sync::Mutex;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("BMF destination honors publication contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    check(&EvidenceBmfAdapter::new(EvidenceBmfConnection {
        output: Mutex::new(Box::new(std::io::sink())),
    }));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
