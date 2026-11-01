//! Neither contestant can certify a case by silently dropping its expected finding.
use super::service::{verify, verify_reference_cases};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::{
        errors::DiagnosticsError as Error,
        types::{BenchmarkFinding, RunReport},
    },
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
            context_manifest: None,
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
    let result = verify_reference_cases(
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
    let empty_cases_error = no_cases.expect_err("empty reference cases must be refused");
    assert!(matches!(empty_cases_error, Error::Invalid(_)));
    assert_eq!(history_len(&execution), Some(0));

    let (suite, finding) = prepared(Scenario::Agreement);
    let launch_reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    let launch_execution = ExecutionFake {
        status: 2,
        ..Default::default()
    };
    let launch_failure = verify_reference_cases(
        &suite,
        &launch_reference,
        &launch_execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    let reference_exit_error = launch_failure.expect_err("nonzero reference exit must be refused");
    assert!(matches!(reference_exit_error, Error::Invalid(_)));
    assert_eq!(history_len(&launch_execution), Some(1));
}

fn history_len(execution: &ExecutionFake) -> Option<usize> {
    execution.history.lock().ok().map(|history| history.len())
}

use crate::domain::measurements::types::{Invocation, ProcessOutput};
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
            project_context: None,
            skipped: Vec::new(),
            coverage: None,
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
            signal: None,
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
            reference_discrepancy: None,
            comparison: crate::domain::suite::types::CaseComparison::Rule,
            rule: ryo::linter::RuleId::try_from("E722").unwrap(),
            name: "positive".into(),
            request: RunRequest {
                context_manifest: None,
                root: ".".into(),
                files: vec![ryo::linter::RelativePath("case.py".into())],
                state: "cache".into(),
            },
            expected: RunReport {
                project_context: None,
                files: vec![ryo::linter::RelativePath("case.py".into())],
                skipped: Vec::new(),
                coverage: None,
                diagnostics: vec![expected.clone()],
            },
        });
    }
    suite.profile = Some(crate::domain::suite::types::Tool {
        project_context_mode: crate::domain::suite::types::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: vec!["E722".try_into().unwrap(), "F901".try_into().unwrap()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
    });
    (suite, expected)
}

/// Candidate fixture exposes complete execution accounting independently of its findings.
struct CoveredCandidate(LinterFake);
impl LinterPort for CoveredCandidate {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        LinterPort::prepare(&self.0, request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        let mut report = LinterPort::decode(&self.0, request, output)?;
        report.coverage = Some(crate::domain::diagnostics::types::RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        });
        Ok(report)
    }
}

/// Explicit fake-port evidence lets orchestration tests distinguish reference and candidate contracts.
fn discrepancy_suite() -> SuiteFake {
    use crate::domain::suite::types::{
        ArtifactDigest, DiscrepancyEvidence, DiscrepancyEvidenceKind, PreparedReferenceDiscrepancy,
        ProjectContextMode, ReferenceDiscrepancyIdentity, Tool,
    };
    let (mut suite, _) = prepared(Scenario::Agreement);
    let case = suite.prepared.first_mut().expect("authored case");
    case.comparison = crate::domain::suite::types::CaseComparison::Full;
    case.request.root = "/case".into();
    let mut reference = case.expected.clone();
    reference.diagnostics.clear();
    let profile_hash = ArtifactDigest("a".repeat(64));
    let binary_hash = ArtifactDigest("b".repeat(64));
    let settings = super::discrepancy::json_digest(&serde_json::json!({
        "arguments": [], "environment": {}, "directory": "$ROOT",
        "parser_configuration": "python-target=3.10", "project_context_mode": "Isolated",
    }));
    case.reference_discrepancy = Some(PreparedReferenceDiscrepancy {
        identity: ReferenceDiscrepancyIdentity {
            tool: "fixture".into(),
            version: "1".into(),
            observed_executable_sha256: binary_hash.clone(),
            profile_sha256: profile_hash.clone(),
            settings_sha256: settings,
            source_sha256: ArtifactDigest("c".repeat(64)),
            intended_output_sha256: super::discrepancy::output_digest(&case.expected),
            reference_output_sha256: super::discrepancy::output_digest(&reference),
        },
        expected: reference,
        reason: "independent language contract differs from the pinned reference".into(),
        evidence: vec![DiscrepancyEvidence {
            path: "evidence.json".into(),
            sha256: ArtifactDigest("d".repeat(64)),
            kind: DiscrepancyEvidenceKind::IndependentLanguageRun,
            language_version: "3.10".into(),
            target_configuration: "python-target=3.10".into(),
            version_rationale: "exact target independently evaluated".into(),
        }],
    });
    suite.profile = Some(Tool {
        project_context_mode: ProjectContextMode::Isolated,
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![binary_hash.0.clone().into()],
        defaults: vec![case.rule.clone()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: Some("python-target=3.10".into()),
    });
    let _previous_profile = suite
        .artifact_digests
        .insert("/profile/benchmark.toml".into(), profile_hash);
    let _previous_binary = suite.artifact_digests.insert("fixture".into(), binary_hash);
    suite.discrepancy_artifacts_valid = true;
    let authored_case = suite.prepared.first_mut().expect("authored case");
    let item = authored_case
        .reference_discrepancy
        .take()
        .expect("fixture canonical certificate");
    let identity = item.identity;
    let discrepancy = serde_json::from_value(serde_json::json!({
        "expected": [], "reason": item.reason,
        "identity": { "tool": identity.tool.to_string(), "version": identity.version.to_string(),
            "observed-executable-sha256": identity.observed_executable_sha256.0, "profile-sha256": identity.profile_sha256.0,
            "settings-sha256": identity.settings_sha256.0, "source-sha256": identity.source_sha256.0,
            "reference-output-sha256": identity.reference_output_sha256.0, "intended-output-sha256": identity.intended_output_sha256.0 },
        "evidence": [{ "path": "evidence.json", "sha256": "d".repeat(64), "kind": "independent-language-run",
            "language-version": "3.10", "target-configuration": "python-target=3.10", "version-rationale": "exact target independently evaluated" }],
    })).expect("boundary-owned raw wire certificate");
    suite.reference_wires.push(
        crate::outbound::suite::types::discrepancy::CaseReferenceWire {
            rule: authored_case.rule.to_string(),
            name: authored_case.name.to_string(),
            discrepancy,
        },
    );
    suite
}

#[test]
fn native_both_sides_require_intended_behavior_even_when_wrong_oracle_matches() {
    let suite = discrepancy_suite();
    let reference = LinterFake::default();
    let intended = CoveredCandidate(LinterFake {
        diagnostics: suite.prepared[0].expected.diagnostics.clone(),
        ..Default::default()
    });
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    let rules = Path::new("/profile/rules");
    assert_eq!(
        verify(
            &suite,
            &reference,
            &intended,
            &execution,
            rules,
            Path::new("scratch")
        )
        .expect("distinct exact expectations both hold")
        .0,
        1
    );
    assert_eq!(
        execution.history.lock().expect("observed calls").len(),
        2,
        "both actual sides executed"
    );
    let wrong_oracle = CoveredCandidate(LinterFake::default());
    assert!(
        verify(
            &suite,
            &reference,
            &wrong_oracle,
            &execution,
            rules,
            Path::new("scratch")
        )
        .is_err(),
        "agreement with incorrect oracle must fail intended contract"
    );
    let mut canonical = suite.prepared.clone();
    super::discrepancy::attach(&suite, rules, &mut canonical)
        .expect("application owns wire conversion");
    assert!(
        !super::discrepancy::matches_intended(
            &canonical[0],
            super::discrepancy::reference_expected(&canonical[0])
        ),
        "JSONL retains raw reference mismatch"
    );
}

#[test]
fn native_reference_only_checks_the_distinct_pinned_reference_expectation() {
    let suite = discrepancy_suite();
    let reference = LinterFake::default();
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    assert_eq!(
        verify_reference_cases(
            &suite,
            &reference,
            &execution,
            Path::new("/profile/rules"),
            Path::new("scratch")
        )
        .expect("pinned reference expectation holds")
        .0,
        1
    );
    let wrong_reference = LinterFake {
        diagnostics: suite.prepared[0].expected.diagnostics.clone(),
        ..Default::default()
    };
    assert!(
        verify_reference_cases(
            &suite,
            &wrong_reference,
            &execution,
            Path::new("/profile/rules"),
            Path::new("scratch")
        )
        .is_err(),
        "intended candidate output cannot replace pinned reference behavior"
    );
    let mut stale = suite;
    stale.discrepancy_artifacts_valid = false;
    assert!(
        verify_reference_cases(
            &stale,
            &reference,
            &execution,
            Path::new("/profile/rules"),
            Path::new("scratch")
        )
        .is_err(),
        "reference-only path cannot bypass independent evidence"
    );
}
