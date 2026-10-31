//! Neither contestant can certify a case by silently dropping its expected finding.
use super::service::verify;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::types::{BenchmarkFinding, RunReport},
    suite::types::{PreparedCase, RunRequest},
};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
    suite::fixtures::fake::SuiteFake,
};
use std::path::Path;

#[derive(Clone, Copy)]
enum Scenario {
    Agreement,
    MissingCandidate,
    MissingReference,
    NoCases,
    OmittedInput,
}
const CASES: &[Case<Scenario, usize, &'static str>] = &[
    Case {
        name: CaseName("both contestants establish the independent expectation"),
        word: Word::Succeeds,
        input: || Scenario::Agreement,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("candidate omissions cannot certify"),
        word: Word::Refuses,
        input: || Scenario::MissingCandidate,
        expect: Expect::Error("candidate mismatch"),
    },
    Case {
        name: CaseName("reference omissions cannot certify"),
        word: Word::Refuses,
        input: || Scenario::MissingReference,
        expect: Expect::Error("reference mismatch"),
    },
    Case {
        name: CaseName("empty case sources refuse certification"),
        word: Word::Refuses,
        input: || Scenario::NoCases,
        expect: Expect::Error("no cases"),
    },
    Case {
        name: CaseName("reported input omissions refuse observations"),
        word: Word::Refuses,
        input: || Scenario::OmittedInput,
        expect: Expect::Error("input coverage"),
    },
];
fn subject(scenario: Scenario) -> Result<usize, &'static str> {
    if matches!(scenario, Scenario::OmittedInput) {
        let request = RunRequest {
            root: ".".into(),
            state: "cache".into(),
            files: vec![ryo::linter::RelativePath("input.py".into())],
        };
        return super::service::run(&Empty, &Empty, &request)
            .map(|_| 0)
            .map_err(|error| {
                assert_eq!(
                    error.to_string(),
                    "tool input coverage differs from requested corpus"
                );
                "input coverage"
            });
    }
    let expected = BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("E722").unwrap(),
        file: ryo::linter::RelativePath("case.py".into()),
        start: crate::domain::diagnostics::types::SourceByteOffset(0),
        end: crate::domain::diagnostics::types::SourceByteOffset(6),
    };
    let mut suite = SuiteFake::default();
    if !matches!(scenario, Scenario::NoCases) {
        suite.prepared.push(PreparedCase {
            rule: ryo::linter::RuleId::try_from("E722").unwrap(),
            name: "positive".into(),
            request: RunRequest {
                root: ".".into(),
                files: vec![ryo::linter::RelativePath("case.py".into())],
                state: "cache".into(),
            },
            expected: RunReport {
                files: vec![ryo::linter::RelativePath("case.py".into())],
                diagnostics: vec![expected.clone()],
            },
        });
    }
    let reference = LinterFake {
        diagnostics: if matches!(scenario, Scenario::MissingReference) {
            Vec::new()
        } else {
            vec![expected.clone()]
        },
        ..Default::default()
    };
    let candidate = LinterFake {
        diagnostics: if matches!(scenario, Scenario::MissingCandidate) {
            Vec::new()
        } else {
            vec![expected]
        },
        ..Default::default()
    };
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    let result = verify(
        &suite,
        &reference,
        &candidate,
        &execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    if !matches!(scenario, Scenario::NoCases) {
        assert_eq!(execution.history.lock().expect("history").len(), 2);
    }
    result.map(|count| count.0).map_err(|error| {
        let message = error.to_string();
        if message.contains("expected-vs-candidate") {
            "candidate mismatch"
        } else if message.contains("expected-vs-reference") {
            "reference mismatch"
        } else if message == "no executable rule cases" {
            "no cases"
        } else {
            "unexpected error"
        }
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

use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{Invocation, ProcessOutput},
};
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};
struct Empty;
impl LinterPort for Empty {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: "unused".into(),
            arguments: vec![],
            environment: Default::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, _: &RunRequest, _: ProcessOutput) -> Result<RunReport, Error> {
        Ok(RunReport {
            diagnostics: vec![],
            files: vec![],
        })
    }
}
impl ExecutionPort for Empty {
    fn run(&self, _: Invocation) -> Result<ProcessOutput, Error> {
        Ok(ProcessOutput {
            status: crate::domain::measurements::types::ProcessExitCode(0),
            stdout: crate::domain::measurements::types::CapturedBytes(vec![]),
            stderr: crate::domain::measurements::types::CapturedBytes(vec![]),
            wall: std::time::Duration::ZERO,
            resources: crate::domain::measurements::types::ResourceObservation::WallOnly,
        })
    }
}
