//! Independent parent and child endpoints agree on the wire protocol and stream failures propagate.
use super::adapter::WorkerJsonAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::errors::Error, measurements::types::ProcessOutput, suite::types::RunRequest,
};
use crate::outbound::{
    linter::{factory, interface::LinterPort},
    worker::{
        fixtures::{contract::check, data::report},
        interface::WorkerPort,
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
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
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
            .extend_from_slice(bytes);
        Ok(bytes.len())
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
        _ => Refusal::Unexpected,
    }
}
fn subject(operation: Input) -> Result<Value, Failure> {
    let capture = Arc::new(Mutex::new(Capture::default()));
    let output = Output {
        capture: capture.clone(),
        fail_write: matches!(operation, Operation::WriteFailure),
        fail_flush: matches!(operation, Operation::FlushFailure),
    };
    let parent = factory::ryo("worker".into(), "rules".into());
    let request = RunRequest {
        root: "workspace".into(),
        files: vec!["source.txt".into()],
        cache: "cache".into(),
    };
    let payload = match operation {
        Operation::Malformed => "{".into(),
        Operation::RoundTrip => parent.prepare(&request).map_err(classify)?.arguments[1].clone(),
        _ => {
            r#"{"root":"workspace","rules":"rules","files":["source.txt"],"cache":"cache"}"#.into()
        }
    };
    let child = WorkerJsonAdapter::new(payload, Box::new(output));
    match operation {
        Operation::Malformed => child.receive().map(|_| false).map_err(classify),
        Operation::WriteFailure | Operation::FlushFailure => {
            child.respond(&report()).map(|()| false).map_err(classify)
        }
        Operation::Contract | Operation::RoundTrip => {
            if matches!(operation, Operation::Contract) {
                check(&child);
            } else {
                let received = child.receive().map_err(classify)?;
                assert_eq!(received.root, request.root);
                assert_eq!(received.rules, std::path::PathBuf::from("rules"));
                assert_eq!(received.files, request.files);
                assert_eq!(received.cache, Some(request.cache.join("ryo")));
                child.respond(&report()).map_err(classify)?;
            }
            let capture = capture.lock().expect("capture");
            assert!(capture.bytes.ends_with(b"\n"));
            assert_eq!(capture.flushes, 1);
            let decoded = parent
                .decode(
                    &request,
                    ProcessOutput {
                        status: 0,
                        stdout: capture.bytes.clone(),
                        stderr: Vec::new(),
                        wall: Duration::ZERO,
                    },
                )
                .map_err(classify)?;
            Ok(decoded == report())
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
