//! Independently written wire records check validation, not only roundtrips.
use super::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal errno record is actual launch failure"),
        word: Word::Succeeds,
        input: || r#"{"result":"LaunchFailed","errno":2}"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown response fields refused"),
        word: Word::Refuses,
        input: || r#"{"result":"LaunchFailed","errno":2,"extra":1}"#,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("missing resources refused"),
        word: Word::Refuses,
        input: || r#"{"result":"Observed","output":{"status":0}}"#,
        expect: Expect::Error(()),
    },
];
fn subject(wire: Input) -> Result<Value, Failure> {
    let result = observation(&CollectorMessage(wire.as_bytes().to_vec())).map_err(|_| ())?;
    Ok(matches!(result,Err(Error::Io(error)) if error.kind()==std::io::ErrorKind::NotFound))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
