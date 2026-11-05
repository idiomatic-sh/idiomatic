//! Every codec preserves native inputs, resources, and actual OS launch errors.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::collection::{interface::CollectionPort, types::CollectorMessage};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::measurements::types::ResidentBytes;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use ryo_benchmark_contracts::domain::measurements::types::Resources;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::time::Duration;
/// Independently specified protocol observations.
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    /// Native request fields and bytes.
    Invocation,
    /// Measured clocks, memory, streams and exit code.
    Observation,
    /// A raw operating-system launch error.
    LaunchFailure,
    /// Malformed input is refused.
    MalformedRequest,
    /// Malformed output is refused.
    MalformedResponse,
}
/// Protocol operation to exercise.
type Input = Operation;
/// Whether decoded protocol values match the independently specified observation.
type Value = bool;
/// Human-readable protocol failure.
type Failure = String;
/// Each pair exercises both encoding and decoding methods.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("native invocation roundtrip"),
        word: Word::Succeeds,
        input: || Operation::Invocation,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("resource units and streams roundtrip"),
        word: Word::Succeeds,
        input: || Operation::Observation,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("launch errno survives protocol"),
        word: Word::Succeeds,
        input: || Operation::LaunchFailure,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("malformed request refused"),
        word: Word::Succeeds,
        input: || Operation::MalformedRequest,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("malformed response refused"),
        word: Word::Succeeds,
        input: || Operation::MalformedResponse,
        expect: Expect::Value(true),
    },
];
/// Check all protocol methods and refusal behavior.
/// # Panics
/// Panics if any contract observation differs.
pub fn check(codec: &impl CollectionPort) {
    let subject = |operation| -> Result<bool, String> {
        observe(codec, operation).map_err(|error| error.to_string())
    };
    run(CASES, Word::Succeeds, subject);
}
/// Exercise one protocol roundtrip against independently constructed values.
fn observe(codec: &impl CollectionPort, operation: Operation) -> Result<bool, Error> {
    match operation {
        Operation::Invocation => {
            let mut call = Invocation {
                executable: "/absolute/tool".into(),
                arguments: vec!["argument with spaces".into()],
                environment: std::collections::BTreeMap::default(),
                directory: "/work tree".into(),
            };
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStringExt as _;
                call.arguments
                    .push(std::ffi::OsString::from_vec(vec![255, 128, 65]));
            }
            #[cfg(not(unix))]
            call.arguments.push("portable argument".into());
            let _previous = call.environment.insert("SETTING".into(), "value".into());
            Ok(codec.invocation(&codec.request(&call)?)? == call)
        }
        Operation::Observation => {
            let resources = Resources {
                cpu: Duration::from_millis(123),
                peak_rss_bytes: ResidentBytes(456_789),
            };
            let response = codec.response(Ok(ProcessOutput {
                signal: None,
                status: ProcessExitCode(127),
                stdout: CapturedBytes(vec![255, 0, 7]),
                stderr: CapturedBytes(vec![128, 3]),
                wall: Duration::from_millis(234),
                resources: ResourceObservation::Collected(resources),
            }))?;
            let output = codec.observation(&response)??;
            Ok(output.status.0 == 127
                && output.stdout.0 == [255, 0, 7]
                && output.stderr.0 == [128, 3]
                && output.wall == Duration::from_millis(234)
                && output.resources == ResourceObservation::Collected(resources))
        }
        Operation::LaunchFailure => {
            let response = codec.response(Err(std::io::Error::from_raw_os_error(8).into()))?;
            Ok(
                matches!(codec.observation(&response)?,Err(Error::Io(error)) if error.raw_os_error()==Some(8)),
            )
        }
        Operation::MalformedRequest => {
            Ok(codec.invocation(&CollectorMessage(b"{}".to_vec())).is_err())
        }
        Operation::MalformedResponse => Ok(codec
            .observation(&CollectorMessage(b"{}".to_vec()))
            .is_err()),
    }
}
