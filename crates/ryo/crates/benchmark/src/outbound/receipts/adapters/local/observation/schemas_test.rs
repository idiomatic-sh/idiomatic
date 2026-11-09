//! What the observation document writes, one row of `CASES` for each input.
use super::ObservationSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::receipts::fixtures::samples::observation;

type Input = ();
type Value = bool;
type Failure = ();

/// A row names the rule and case it observed.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a row is written with the rule it observed"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Encodes one sample row and answers the rule it wrote.
fn subject(_input: Input) -> Result<Value, Failure> {
    let schema = ObservationSchema::from(&observation());
    let row = serde_json::to_value(&schema).map_err(|_refused| ())?;
    Ok(row["rule"] == "R1")
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
