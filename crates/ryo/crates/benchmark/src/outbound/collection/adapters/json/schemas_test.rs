//! Independently written wire records check validation, not only roundtrips.
use super::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
type Value = Observed;
type Failure = ();
/// What the collector protocol answers for a wire record.
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    /// The record is an actual launch failure of a missing program.
    LaunchNotFound,
    /// The record is a signaled child whose resources survive an incomplete report.
    Signaled {
        signal: TerminationSignal,
        status: i32,
        wall: Duration,
        stdout: [u8; 1],
        peak_rss_bytes: u64,
        roundtrip_signal: TerminationSignal,
    },
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal errno record is actual launch failure"),
        word: Word::Succeeds,
        input: || r#"{"result":"LaunchFailed","errno":2}"#,
        expect: Expect::Value(Observed::LaunchNotFound),
    },
    Case {
        name: CaseName("signal resources survive incomplete report"),
        word: Word::Succeeds,
        input: || r#"{"result":"Observed","output":{"status":-15,"signal":15,"stdout":[123],"stderr":[],"wall":{"secs":2,"nanos":0},"cpu":{"secs":1,"nanos":0},"rss":12345}}"#,
        expect: Expect::Value(Observed::Signaled {
            signal: TerminationSignal::Signalled(SignalNumber(15_i32)),
            status: -15_i32,
            wall: Duration::from_secs(2),
            stdout: *b"{",
            peak_rss_bytes: 12345,
            roundtrip_signal: TerminationSignal::Signalled(SignalNumber(15_i32)),
        }),
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
    let message = CollectorMessage(wire.as_bytes().to_vec());
    let result = observation(&message).map_err(|_invalid_boundary_value| ())?;
    let output = match result {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Observed::LaunchNotFound);
        }
        Err(_other_failure) => return Err(()),
        Ok(output) => output,
    };
    let ResourceObservation::Collected(Resources {
        peak_rss_bytes: ResidentBytes(peak_rss_bytes),
        ..
    }) = output.resources
    else {
        return Err(());
    };
    let observed = (
        output.signal,
        output.status.0,
        output.wall,
        output.stdout.0.clone(),
    );
    let encoded = response(Ok(output)).map_err(|_invalid_boundary_value| ())?;
    let roundtrip_signal = observation(&encoded)
        .map_err(|_invalid_boundary_value| ())?
        .map_err(|_invalid_boundary_value| ())?
        .signal;
    Ok(Observed::Signaled {
        signal: observed.0,
        status: observed.1,
        wall: observed.2,
        stdout: observed
            .3
            .as_slice()
            .try_into()
            .map_err(|_wrong_length| ())?,
        peak_rss_bytes,
        roundtrip_signal,
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
