//! Independently written wire records check validation, not only roundtrips.
use super::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("native invocation request bytes preserve argument boundaries"),
        word: Word::Succeeds,
        input: || "request",
        expect: Expect::Value(true),
    },
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
    if wire == "request" {
        let encoded = request(&Invocation {
            executable: std::path::PathBuf::default(),
            arguments: vec!["a".into()],
            environment: std::collections::BTreeMap::default(),
            directory: std::path::PathBuf::default(),
        })
        .map_err(|_invalid_boundary_value| ())?;
        let actual: serde_json::Value =
            serde_json::from_slice(&encoded.0).map_err(|_invalid_boundary_value| ())?;
        let platform = if cfg!(windows) { "Windows" } else { "Unix" };
        let expected = serde_json::json!({"executable":{platform:[]},"arguments":[{platform:[97_u8]}],"environment":[],"directory":{platform:[]}});
        return Ok(actual == expected);
    }
    let result = observation(&CollectorMessage(wire.as_bytes().to_vec()))
        .map_err(|_invalid_boundary_value| ())?;
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
