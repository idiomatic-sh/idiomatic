//! A registered case read from Python's registration is written back in the same wire form.
use super::case_value;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::types::RegistrationBytes;
use crate::outbound::inventory::types::Registration;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;

/// The checked-in registration a row reads.
type Input = &'static str;
/// Whether every case written back equals the case read, and how many there are.
type Value = (bool, usize);
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a case of the python registration is written back as it was read"),
    word: Word::Succeeds,
    input: || "receipts/python-registration.json",
    expect: Expect::Value((true, 1)),
}];

/// Reads the registration and writes each of its cases back.
fn subject(path: Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    let text = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/").to_owned() + path,
    )
    .map_err(|error| leak(error.to_string()))?;
    let read: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| leak(error.to_string()))?;
    let registration = Registration::try_from(RegistrationBytes(&CapturedBytes(text.into_bytes())))
        .map_err(|error| leak(error.to_string()))?;
    let mut count = 0;
    let mut same = true;
    for (rule, written) in registration.rules.iter().enumerate() {
        for (index, case) in written.cases.iter().enumerate() {
            let value = case_value(case).map_err(|error| leak(error.to_string()))?;
            same &= value == read["rules"][rule]["cases"][index];
            count += 1;
        }
    }
    Ok((same, count))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
