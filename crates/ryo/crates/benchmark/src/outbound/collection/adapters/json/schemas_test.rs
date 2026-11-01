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

#[test]
fn signal_resources_survive_incomplete_report() -> Result<(), Box<dyn std::error::Error>> {
    let wire = CollectorMessage(br#"{"result":"Observed","output":{"status":-15,"signal":15,"stdout":[123],"stderr":[],"wall":{"secs":2,"nanos":0},"cpu":{"secs":1,"nanos":0},"rss":12345}}"#.to_vec());
    let output = observation(&wire)??;
    assert_eq!(output.signal, Some(15_i32));
    assert_eq!(output.status.0, -15_i32);
    assert_eq!(output.wall, Duration::from_secs(2));
    assert_eq!(output.stdout.0, b"{");
    assert!(matches!(
        output.resources,
        ResourceObservation::Collected(Resources {
            peak_rss_bytes: ResidentBytes(12345),
            ..
        })
    ));
    let encoded = response(Ok(output))?;
    assert_eq!(observation(&encoded)??.signal, Some(15_i32));
    Ok(())
}
