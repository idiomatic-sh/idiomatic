//! Independent parent and child endpoints agree on the wire protocol and stream failures propagate.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::{
    linter::{factory, interface::LinterPort as _},
    worker::{
        fixtures::{contract::check, data::report},
        interface::WorkerPort as _,
    },
};
use benchmark_contracts::domain::measurements::types::ProcessOutput;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::types::RunRequest;
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
        if matches!(operation, Operation::RoundTrip) {
            std::collections::BTreeMap::from([
                (
                    benchmark_contracts::domain::requests::types::RuleLabel("fixture".into()),
                    benchmark_contracts::domain::requests::types::ApplicabilityPolicy(
                        "always".into(),
                    ),
                ),
                (
                    benchmark_contracts::domain::requests::types::RuleLabel("other".into()),
                    benchmark_contracts::domain::requests::types::ApplicabilityPolicy(
                        "accepted-syntax".into(),
                    ),
                ),
            ])
        } else {
            std::collections::BTreeMap::default()
        },
        benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        vec![ryo::linter::RuleId::try_from("fixture").unwrap()],
        std::collections::BTreeMap::new().into(),
    );
    let temporary = tempfile::tempdir().map_err(|_invalid_parser| Refusal::Unexpected)?;
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations:
            benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata:
            benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: "workspace".into(),
        files: vec![ryo::linter::RelativePath("source.txt".into())],
        state: temporary.path().join("cache"),
    };
    let payload: String = match operation {
        Operation::Malformed => "{".into(),
        Operation::RoundTrip => {
            let invocation = parent.prepare(&request).map_err(classify)?;
            let request_path = invocation.arguments.get(2).ok_or(Refusal::Unexpected)?;
            std::fs::read_to_string(std::path::PathBuf::from(request_path))
                .map_err(|_invalid_parser| Refusal::Unexpected)?
        }
        Operation::Contract | Operation::WriteFailure | Operation::FlushFailure => {
            r#"{"root":"workspace","rules":"rules","files":["source.txt"],"cache":"cache"}"#.into()
        }
    };
    let payload_text: String = payload.clone();
    let child = crate::outbound::worker::factory::json(
        crate::outbound::worker::types::WorkerSource::Argument(payload.into()),
        Box::new(output),
    );
    let mut expected_report = report();
    if matches!(operation, Operation::RoundTrip) {
        let skipped_rule = ryo::linter::RuleId::try_from("other").expect("fixture rule");
        expected_report.skipped.push(ryo::linter::Skipped {
            rule: skipped_rule,
            path: ryo::linter::RelativePath("source.txt".into()),
            reason: ryo::linter::SkipReason::SyntaxRejected,
        });
        let benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(coverage) =
            &mut expected_report.coverage
        else {
            panic!("complete fixture coverage");
        };
        coverage.selected = benchmark_contracts::domain::reports::types::PairCount(2);
        coverage.skipped = benchmark_contracts::domain::reports::types::PairCount(1);
    }
    match operation {
        Operation::Malformed => child.receive().map(|_| false).map_err(classify),
        Operation::WriteFailure | Operation::FlushFailure => child
            .respond(&expected_report)
            .map(|()| false)
            .map_err(classify),
        Operation::Contract | Operation::RoundTrip => {
            if matches!(operation, Operation::Contract) {
                let stored = temporary.path().join("request.json");
                std::fs::write(&stored, payload_text.as_bytes())
                    .map_err(|_unwritten| Refusal::Unexpected)?;
                check(&child, &stored);
            } else {
                let received = child.receive().map_err(classify)?;
                // The same request stored in a file reads back as the one this process was given.
                let stored = temporary.path().join("request.json");
                std::fs::write(&stored, payload_text.as_bytes())
                    .map_err(|_unwritten| Refusal::Unexpected)?;
                let read = child.read(&stored).map_err(classify)?;
                assert_eq!(
                    (&read.root, &read.files, &read.state),
                    (&received.root, &received.files, &received.state)
                );
                assert_eq!(received.root, request.root);
                assert_eq!(received.rules, std::path::PathBuf::from("rules"));
                assert_eq!(received.files, request.files);
                assert_eq!(
                    received
                        .rule_applicability
                        .get(&benchmark_contracts::domain::requests::types::RuleLabel(
                            "other".into()
                        ))
                        .map(|policy| policy.0.as_str()),
                    Some("accepted-syntax")
                );
                assert_eq!(
                    received.state,
                    benchmark_contracts::domain::requests::types::WorkerState::Retained(
                        request.state.join("ryo")
                    )
                );
                child.respond(&expected_report).map_err(classify)?;
            }
            let capture = capture.lock().expect("capture");
            if matches!(operation, Operation::RoundTrip) {
                // What a worker in the bench's own process answers is what the child publishes.
                assert_eq!(
                    child.encoded(&expected_report).map_err(classify)?.0,
                    capture.bytes
                );
            }
            assert!(capture.bytes.ends_with(b"\n"));
            assert_eq!(capture.flushes, 1);
            let decoded = parent
                .decode(
                    &request,
                    ProcessOutput {
                        signal: benchmark_contracts::domain::measurements::types::TerminationSignal::Exited,
                        status: benchmark_contracts::domain::measurements::types::ProcessExitCode(0),
                        stdout: benchmark_contracts::domain::measurements::types::CapturedBytes(
                            capture.bytes.clone(),
                        ),
                        stderr: benchmark_contracts::domain::measurements::types::CapturedBytes(Vec::new()),
                        wall: Duration::ZERO,
                        resources:
                            benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly,
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
