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
    MissingQuery,
    StartupFailure,
    FullAgreement,
    MissingFrontend,
    RuleProjection,
}
const CASES: &[Case<Scenario, usize, &'static str>] = &[
    Case {
        name: CaseName("full report preserves distinct frontend and lint identities"),
        word: Word::Succeeds,
        input: || Scenario::FullAgreement,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("matching lint cannot conceal a missing frontend diagnostic"),
        word: Word::Refuses,
        input: || Scenario::MissingFrontend,
        expect: Expect::Error("candidate mismatch"),
    },
    Case {
        name: CaseName("rule scope explicitly excludes unrelated diagnostics"),
        word: Word::Succeeds,
        input: || Scenario::RuleProjection,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("candidate startup failure executes once before fixture comparisons"),
        word: Word::Refuses,
        input: || Scenario::StartupFailure,
        expect: Expect::Error("candidate startup"),
    },
    Case {
        name: CaseName("removed query refuses before either contestant executes"),
        word: Word::Refuses,
        input: || Scenario::MissingQuery,
        expect: Expect::Error("unaccounted default"),
    },
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
    let (mut suite, expected) = prepared(scenario);
    let mut reference = LinterFake {
        diagnostics: if matches!(scenario, Scenario::MissingReference) {
            Vec::new()
        } else {
            vec![expected.clone()]
        },
        ..Default::default()
    };
    let mut candidate = LinterFake {
        diagnostics: if matches!(scenario, Scenario::MissingCandidate) {
            Vec::new()
        } else {
            vec![expected]
        },
        ..Default::default()
    };
    report_scope(scenario, &mut suite, &mut reference, &mut candidate);
    let execution = ExecutionFake {
        status: if matches!(scenario, Scenario::StartupFailure) {
            2
        } else {
            0
        },
        ..Default::default()
    };
    let result = if matches!(scenario, Scenario::MissingQuery) {
        super::service::verify_accounted(
            &suite,
            &reference,
            &candidate,
            &execution,
            Path::new("profile"),
            Path::new("scratch"),
        )
    } else {
        verify(
            &suite,
            &reference,
            &candidate,
            &execution,
            Path::new("rules"),
            Path::new("scratch"),
        )
    };
    if matches!(scenario, Scenario::MissingQuery) {
        assert!(execution.history.lock().expect("history").is_empty());
    } else if matches!(scenario, Scenario::StartupFailure) {
        assert_eq!(execution.history.lock().expect("history").len(), 1);
    } else if !matches!(scenario, Scenario::NoCases) {
        assert_eq!(execution.history.lock().expect("history").len(), 2);
    } else {
        assert!(
            execution.history.lock().expect("history").is_empty(),
            "no cases must launch no contestants"
        );
    }
    result.map(|count| count.0).map_err(|error| {
        let message = error.to_string();
        if message.contains("expected-vs-candidate") {
            "candidate mismatch"
        } else if message.contains("expected-vs-reference") {
            "reference mismatch"
        } else if message == "operational failure" {
            "candidate startup"
        } else if message.contains("defaults lack implementation or explicit deferral") {
            "unaccounted default"
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
            environment: std::collections::BTreeMap::default(),
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
    fn profile(&self) -> crate::domain::measurements::types::ExecutionProfile {
        crate::domain::measurements::types::ExecutionProfile::Wall
    }
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

/// Give both contestants explicit frontend findings without changing rule-only expectations.
fn report_scope(
    scenario: Scenario,
    suite: &mut SuiteFake,
    reference: &mut LinterFake,
    candidate: &mut LinterFake,
) {
    if matches!(
        scenario,
        Scenario::FullAgreement | Scenario::MissingFrontend | Scenario::RuleProjection
    ) {
        let frontend = BenchmarkFinding {
            rule: "invalid-syntax"
                .try_into()
                .expect("frontend diagnostic identity"),
            file: ryo::linter::RelativePath("case.py".into()),
            start: crate::domain::diagnostics::types::SourceByteOffset(9),
            end: crate::domain::diagnostics::types::SourceByteOffset(19),
        };
        reference.diagnostics.push(frontend.clone());
        if !matches!(scenario, Scenario::MissingFrontend) {
            candidate.diagnostics.push(frontend.clone());
        }
        if !matches!(scenario, Scenario::RuleProjection) {
            let case = suite.prepared.first_mut().expect("authored case");
            case.comparison = crate::domain::suite::types::CaseComparison::Full;
            case.expected.diagnostics.push(frontend);
        }
    }
}

/// Construct authored evidence independently from either contestant response.
fn prepared(scenario: Scenario) -> (SuiteFake, BenchmarkFinding) {
    let expected = BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("E722").unwrap(),
        file: ryo::linter::RelativePath("case.py".into()),
        start: crate::domain::diagnostics::types::SourceByteOffset(0),
        end: crate::domain::diagnostics::types::SourceByteOffset(6),
    };
    let mut suite = SuiteFake::default();
    if !matches!(scenario, Scenario::NoCases) {
        suite.prepared.push(PreparedCase {
            comparison: crate::domain::suite::types::CaseComparison::Rule,
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
    suite.profile = Some(crate::domain::suite::types::Tool {
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: vec!["E722".try_into().unwrap(), "F901".try_into().unwrap()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
    });
    (suite, expected)
}
