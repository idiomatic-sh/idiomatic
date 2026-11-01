//! Independent parent and child endpoints agree on the wire protocol and stream failures propagate.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error, measurements::types::ProcessOutput,
    suite::types::RunRequest,
};
use crate::outbound::{
    linter::{factory, interface::LinterPort as _},
    worker::{
        fixtures::{contract::check, data::report},
        interface::WorkerPort as _,
    },
};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Duration,
};
#[derive(Clone, Copy, Debug)]
enum Operation {
    Contract,
    RoundTrip,
    Malformed,
    WriteFailure,
    FlushFailure,
}
type Input = Operation;
type Value = bool;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    InvalidRequest,
    Write,
    Flush,
    Unexpected,
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("JSON adapter satisfies typed transport contract"),
        word: Word::Succeeds,
        input: || Operation::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("independent parent and child schemas round trip duplicate findings"),
        word: Word::Succeeds,
        input: || Operation::RoundTrip,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("malformed request is not delivered to engine"),
        word: Word::Refuses,
        input: || Operation::Malformed,
        expect: Expect::Error(Refusal::InvalidRequest),
    },
    Case {
        name: CaseName("write failure is not successful publication"),
        word: Word::Fails,
        input: || Operation::WriteFailure,
        expect: Expect::Error(Refusal::Write),
    },
    Case {
        name: CaseName("flush failure is not successful publication"),
        word: Word::Fails,
        input: || Operation::FlushFailure,
        expect: Expect::Error(Refusal::Flush),
    },
];
#[derive(Debug, Default)]
struct Capture {
    bytes: Vec<u8>,
    flushes: usize,
}
#[derive(Clone, Debug)]
struct Output {
    capture: Arc<Mutex<Capture>>,
    fail_write: bool,
    fail_flush: bool,
}
impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.fail_write {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture write failure",
            ));
        }
        self.capture
            .lock()
            .expect("capture")
            .bytes
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            return Err(io::Error::other("fixture flush failure"));
        }
        self.capture.lock().expect("capture").flushes += 1;
        Ok(())
    }
}
fn classify(error: Error) -> Refusal {
    match error {
        Error::Invalid(_) => Refusal::InvalidRequest,
        Error::Io(error)
            if error.kind() == io::ErrorKind::BrokenPipe
                && error.to_string() == "fixture write failure" =>
        {
            Refusal::Write
        }
        Error::Io(error) if error.to_string() == "fixture flush failure" => Refusal::Flush,
        Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Unexpected,
    }
}
fn subject(operation: Input) -> Result<Value, Failure> {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let output = Output {
        capture: Arc::clone(&capture),
        fail_write: matches!(operation, Operation::WriteFailure),
        fail_flush: matches!(operation, Operation::FlushFailure),
    };
    let parent = factory::ryo(
        "worker".into(),
        "rules".into(),
        std::collections::BTreeMap::default(),
        std::collections::BTreeMap::default(),
        None,
    );
    let temporary = tempfile::tempdir().map_err(|_invalid_parser| Refusal::Unexpected)?;
    let request = RunRequest {
        context_manifest: None,
        root: "workspace".into(),
        files: vec![ryo::linter::RelativePath("source.txt".into())],
        state: temporary.path().join("cache"),
    };
    let payload = match operation {
        Operation::Malformed => "{".into(),
        Operation::RoundTrip => {
            let invocation = parent.prepare(&request).map_err(classify)?;
            let request_path = invocation.arguments.get(2).ok_or(Refusal::Unexpected)?;
            std::fs::read_to_string(std::path::PathBuf::from(request_path))
                .map_err(|_invalid_parser| Refusal::Unexpected)?
                .into()
        }
        Operation::Contract | Operation::WriteFailure | Operation::FlushFailure => {
            r#"{"root":"workspace","rules":"rules","files":["source.txt"],"cache":"cache"}"#.into()
        }
    };
    let child = crate::outbound::worker::factory::json(payload, Box::new(output));
    let mut expected_report = report();
    if matches!(operation, Operation::RoundTrip) {
        let Some(rule) = expected_report
            .diagnostics
            .first()
            .map(|finding| finding.rule.clone())
        else {
            return Err(Refusal::Unexpected);
        };
        expected_report.skipped.push(ryo::linter::Skipped {
            rule,
            path: ryo::linter::RelativePath("source.txt".into()),
            reason: ryo::linter::SkipReason::SyntaxRejected,
        });
    }
    match operation {
        Operation::Malformed => child.receive().map(|_| false).map_err(classify),
        Operation::WriteFailure | Operation::FlushFailure => child
            .respond(&expected_report)
            .map(|()| false)
            .map_err(classify),
        Operation::Contract | Operation::RoundTrip => {
            if matches!(operation, Operation::Contract) {
                check(&child);
            } else {
                let received = child.receive().map_err(classify)?;
                assert_eq!(received.root, request.root);
                assert_eq!(received.rules, std::path::PathBuf::from("rules"));
                assert_eq!(received.files, request.files);
                assert_eq!(
                    received.state,
                    crate::domain::suite::types::WorkerState::Retained(request.state.join("ryo"))
                );
                child.respond(&expected_report).map_err(classify)?;
            }
            let capture = capture.lock().expect("capture");
            assert!(capture.bytes.ends_with(b"\n"));
            assert_eq!(capture.flushes, 1);
            let decoded = parent
                .decode(
                    &request,
                    ProcessOutput {
                        signal: None,
                        status: crate::domain::measurements::types::ProcessExitCode(0),
                        stdout: crate::domain::measurements::types::CapturedBytes(
                            capture.bytes.clone(),
                        ),
                        stderr: crate::domain::measurements::types::CapturedBytes(Vec::new()),
                        wall: Duration::ZERO,
                        resources:
                            crate::domain::measurements::types::ResourceObservation::WallOnly,
                    },
                )
                .map_err(classify)?;
            Ok(decoded == expected_report)
        }
    }
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
fn fails() {
    run(CASES, Word::Fails, subject);
}
