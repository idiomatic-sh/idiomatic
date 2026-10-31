//! Invalid requests never launch; child failures are encoded rather than mistaken for codec failure.
use super::{interface::CollectUseCase, use_case::CollectUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{
        CapturedBytes, Invocation, ProcessExitCode, ProcessOutput, ResidentBytes,
        ResourceObservation, Resources,
    },
};
use crate::outbound::{
    collection::{self, interface::CollectionPort, types::CollectorMessage},
    execution::interface::ExecutionPort,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Copy, Debug)]
enum Operation {
    Success,
    Malformed,
    LaunchFailure,
}
type Input = Operation;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("actual observation returned by codec"),
        word: Word::Succeeds,
        input: || Operation::Success,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("malformed request prevents child launch"),
        word: Word::Succeeds,
        input: || Operation::Malformed,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("launch failure is a structured response"),
        word: Word::Succeeds,
        input: || Operation::LaunchFailure,
        expect: Expect::Value(true),
    },
];
struct Execution {
    calls: Arc<AtomicUsize>,
    operation: Operation,
}
impl ExecutionPort for Execution {
    fn profile(&self) -> crate::domain::measurements::types::ExecutionProfile {
        crate::domain::measurements::types::ExecutionProfile::Wall
    }
    fn run(&self, _: Invocation) -> Result<ProcessOutput, Error> {
        let _before = self.calls.fetch_add(1, Ordering::SeqCst);
        if matches!(self.operation, Operation::LaunchFailure) {
            return Err(std::io::Error::from_raw_os_error(2).into());
        }
        Ok(ProcessOutput {
            status: ProcessExitCode(126),
            stdout: CapturedBytes(vec![255]),
            stderr: CapturedBytes(Vec::new()),
            wall: Duration::from_millis(10),
            resources: ResourceObservation::Collected(Resources {
                cpu: Duration::from_millis(5),
                peak_rss_bytes: ResidentBytes(12345),
            }),
        })
    }
}
fn subject(operation: Input) -> Result<Value, Failure> {
    let codec = Arc::new(collection::fixtures::fake::CollectionFake::default());
    let calls = Arc::new(AtomicUsize::new(0));
    let collect = CollectUseCaseImpl::new(
        codec.clone(),
        Arc::new(Execution {
            calls: calls.clone(),
            operation,
        }),
    );
    let request = if matches!(operation, Operation::Malformed) {
        CollectorMessage(b"{}".to_vec())
    } else {
        codec
            .request(&Invocation {
                executable: "fixture".into(),
                arguments: Vec::new(),
                environment: Default::default(),
                directory: "/tmp".into(),
            })
            .map_err(|_| ())?
    };
    let response = collect.collect(&request);
    if matches!(operation, Operation::Malformed) {
        return Ok(
            matches!(response, Err(Error::Invalid(message)) if message == "malformed request")
                && calls.load(Ordering::SeqCst) == 0,
        );
    }
    let decoded = codec
        .observation(&response.map_err(|_| ())?)
        .map_err(|_| ())?;
    let expected = match operation {
        Operation::Success => {
            matches!(decoded,Ok(output) if output.status.0==126 && output.stdout.0==[255] && matches!(output.resources,ResourceObservation::Collected(Resources{peak_rss_bytes:ResidentBytes(12345),..})))
        }
        Operation::LaunchFailure => {
            matches!(decoded,Err(Error::Io(error)) if error.raw_os_error()==Some(2))
        }
        Operation::Malformed => false,
    };
    Ok(expected && calls.load(Ordering::SeqCst) == 1)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
