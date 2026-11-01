//! Neither contestant can certify a case by silently dropping its expected finding.
use super::service::{verify, verify_reference_cases};
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
        name: CaseName("rule scope preserves parser diagnostics and excludes unrelated rules"),
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

#[test]
fn reference_only_accepts_exact_full_report_multiplicity_without_candidate() {
    let (mut suite, finding) = prepared(Scenario::Agreement);
    let Some(case) = suite.prepared.first_mut() else {
        assert_eq!(
            suite.prepared.len(),
            1,
            "fixture setup must include its case"
        );
        return;
    };
    case.comparison = crate::domain::suite::types::CaseComparison::Full;
    case.expected.diagnostics = vec![finding.clone(), finding.clone()];
    let reference = LinterFake {
        diagnostics: vec![finding.clone(), finding],
        ..Default::default()
    };
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    let result = verify_reference_cases(
        &suite,
        &reference,
        &execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    assert!(result.is_ok(), "reference fixtures must match: {result:?}");
    if let Ok(count) = result {
        assert_eq!(count.0, 1);
    }
    assert_eq!(history_len(&execution), Some(1));
}

#[test]
fn reference_only_preserves_frontend_findings_in_rule_comparison() {
    let (mut suite, _) = prepared(Scenario::RuleProjection);
    let mut reference = LinterFake {
        diagnostics: vec![
            BenchmarkFinding {
                rule: "E722".try_into().unwrap(),
                file: ryo::linter::RelativePath("case.py".into()),
                start: crate::domain::diagnostics::types::SourceByteOffset(0),
                end: crate::domain::diagnostics::types::SourceByteOffset(6),
            },
            BenchmarkFinding {
                rule: "invalid-syntax".try_into().unwrap(),
                file: ryo::linter::RelativePath("case.py".into()),
                start: crate::domain::diagnostics::types::SourceByteOffset(9),
                end: crate::domain::diagnostics::types::SourceByteOffset(19),
            },
            BenchmarkFinding {
                rule: "F401".try_into().unwrap(),
                file: ryo::linter::RelativePath("case.py".into()),
                start: crate::domain::diagnostics::types::SourceByteOffset(20),
                end: crate::domain::diagnostics::types::SourceByteOffset(24),
            },
        ],
        ..Default::default()
    };
    let mut candidate = LinterFake::default();
    report_scope(
        Scenario::RuleProjection,
        &mut suite,
        &mut reference,
        &mut candidate,
    );
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    let result = verify_reference_cases(
        &suite,
        &reference,
        &execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    assert!(
        result.is_ok(),
        "rule-scoped reference output differs: {result:?}"
    );
    assert_eq!(history_len(&execution), Some(1));
}

#[test]
fn reference_only_rejects_wrong_byte_end_offset() {
    let (mut suite, finding) = prepared(Scenario::Agreement);
    let Some(case) = suite.prepared.first_mut() else {
        assert_eq!(
            suite.prepared.len(),
            1,
            "fixture setup must include its case"
        );
        return;
    };
    case.comparison = crate::domain::suite::types::CaseComparison::Full;
    let Some(expected) = case.expected.diagnostics.first_mut() else {
        assert_eq!(
            case.expected.diagnostics.len(),
            1,
            "fixture setup must include its finding"
        );
        return;
    };
    expected.end = crate::domain::diagnostics::types::SourceByteOffset(7);
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    let error = verify_reference_cases(
        &suite,
        &reference,
        &execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    assert!(result.is_err(), "wrong byte endpoint must fail");
    if let Err(error) = result {
        assert!(error.to_string().contains("expected-vs-reference"));
    }
}

#[test]
fn reference_only_rejects_missing_duplicate_and_unexpected_empty_case_finding() {
    for expected_count in [0, 2] {
        let (mut suite, finding) = prepared(Scenario::Agreement);
        let Some(case) = suite.prepared.first_mut() else {
            assert_eq!(
                suite.prepared.len(),
                1,
                "fixture setup must include its case"
            );
            return;
        };
        case.comparison = crate::domain::suite::types::CaseComparison::Full;
        case.expected.diagnostics = vec![finding.clone(); expected_count];
        let actual = if expected_count == 0 {
            vec![finding]
        } else {
            vec![finding.clone()]
        };
        let reference = LinterFake {
            diagnostics: actual,
            ..Default::default()
        };
        let execution = ExecutionFake {
            status: 0,
            ..Default::default()
        };
        let result = verify_reference_cases(
            &suite,
            &reference,
            &execution,
            Path::new("rules"),
            Path::new("scratch"),
        );
        assert!(
            result.is_err(),
            "reference output must match exact authored multiplicity"
        );
        if let Err(error) = result {
            assert!(error.to_string().contains("expected-vs-reference"));
        }
    }
}

#[test]
fn reference_only_refuses_empty_cases_and_reference_launch_failure() {
    let (empty, _) = prepared(Scenario::NoCases);
    let reference = LinterFake::default();
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    let no_cases = verify_reference_cases(
        &empty,
        &reference,
        &execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    assert!(no_cases.is_err());
    assert_eq!(history_len(&execution), Some(0));

    let (suite, finding) = prepared(Scenario::Agreement);
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    let execution = ExecutionFake {
        status: 2,
        ..Default::default()
    };
    let launch_failure = verify_reference_cases(
        &suite,
        &reference,
        &execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    assert!(launch_failure.is_err());
    assert_eq!(history_len(&execution), Some(1));
}

fn history_len(execution: &ExecutionFake) -> Option<usize> {
    execution.history.lock().ok().map(|history| history.len())
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

/// Give both contestants parser diagnostics, optionally retaining an unrelated lint as a control.
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
        if matches!(scenario, Scenario::RuleProjection) {
            let unrelated = BenchmarkFinding {
                rule: "F401".try_into().expect("unrelated lint identity"),
                file: ryo::linter::RelativePath("case.py".into()),
                start: crate::domain::diagnostics::types::SourceByteOffset(20),
                end: crate::domain::diagnostics::types::SourceByteOffset(24),
            };
            reference.diagnostics.push(unrelated.clone());
            candidate.diagnostics.push(unrelated);
        }
        if !matches!(scenario, Scenario::RuleProjection) {
            suite
                .prepared
                .first_mut()
                .expect("authored case")
                .comparison = crate::domain::suite::types::CaseComparison::Full;
        }
        suite
            .prepared
            .first_mut()
            .expect("authored case")
            .expected
            .diagnostics
            .push(frontend);
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
