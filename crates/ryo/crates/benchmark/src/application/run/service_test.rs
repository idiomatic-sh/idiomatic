//! Neither contestant can certify a case by silently dropping its expected finding.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
    suite::fixtures::fake::SuiteFake,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::workloads::types::{CaseComparison, PreparedCase};
use std::path::Path;

#[derive(Clone, Copy)]
enum Scenario {
    Agreement,
    MissingCandidate,
    MissingReference,
    NoCases,
    OmittedInput,
    StartupFailure,
    FullAgreement,
    MissingFrontend,
    RuleProjection,
    ReferenceMultiplicity,
    ReferenceFrontend,
    ReferenceWrongEnd,
    ReferenceMiscounted { expected_count: usize },
    ReferenceNoCases,
    ReferenceStartup,
    NativeIntended,
    NativeWrongOracle,
    NativeReference,
    NativeWrongReference,
    NativeStaleEvidence,
    ModeDrift { expected: bool, drift: bool },
    MeasuredPlain,
    ObservationBearing,
}
type Input = Scenario;
type Value = usize;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
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
    Case {
        name: CaseName("reference only accepts exact full report multiplicity without a candidate"),
        word: Word::Succeeds,
        input: || Scenario::ReferenceMultiplicity,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("reference only preserves frontend findings in rule comparison"),
        word: Word::Succeeds,
        input: || Scenario::ReferenceFrontend,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("reference only rejects a wrong byte end offset"),
        word: Word::Refuses,
        input: || Scenario::ReferenceWrongEnd,
        expect: Expect::Error("reference mismatch"),
    },
    Case {
        name: CaseName("reference only rejects an unexpected finding in an empty case"),
        word: Word::Refuses,
        input: || Scenario::ReferenceMiscounted { expected_count: 0 },
        expect: Expect::Error("reference mismatch"),
    },
    Case {
        name: CaseName("reference only rejects a duplicate the reference does not report"),
        word: Word::Refuses,
        input: || Scenario::ReferenceMiscounted { expected_count: 2 },
        expect: Expect::Error("reference mismatch"),
    },
    Case {
        name: CaseName("reference only refuses empty cases"),
        word: Word::Refuses,
        input: || Scenario::ReferenceNoCases,
        expect: Expect::Error("no cases"),
    },
    Case {
        name: CaseName("reference only refuses a reference launch failure"),
        word: Word::Refuses,
        input: || Scenario::ReferenceStartup,
        expect: Expect::Error("reference startup"),
    },
    Case {
        name: CaseName("both sides require intended behavior even when a wrong oracle matches"),
        word: Word::Succeeds,
        input: || Scenario::NativeIntended,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("agreement with an incorrect oracle fails the intended contract"),
        word: Word::Refuses,
        input: || Scenario::NativeWrongOracle,
        expect: Expect::Error("candidate mismatch"),
    },
    Case {
        name: CaseName("reference only checks the distinct pinned reference expectation"),
        word: Word::Succeeds,
        input: || Scenario::NativeReference,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("intended candidate output cannot replace pinned reference behavior"),
        word: Word::Refuses,
        input: || Scenario::NativeWrongReference,
        expect: Expect::Error("reference mismatch"),
    },
    Case {
        name: CaseName("reference only cannot bypass independent evidence"),
        word: Word::Refuses,
        input: || Scenario::NativeStaleEvidence,
        expect: Expect::Error("unproven evidence"),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("an unchanged non-executable mode lets both cases execute"),
        word: Word::Succeeds,
        input: || Scenario::ModeDrift { expected: false, drift: false },
        expect: Expect::Value(2),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("an unchanged executable mode lets both cases execute"),
        word: Word::Succeeds,
        input: || Scenario::ModeDrift { expected: true, drift: false },
        expect: Expect::Value(2),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("a reference that makes a file executable stops the second candidate"),
        word: Word::Refuses,
        input: || Scenario::ModeDrift { expected: false, drift: true },
        expect: Expect::Error("metadata drift"),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("a reference that clears the executable bit stops the second candidate"),
        word: Word::Refuses,
        input: || Scenario::ModeDrift { expected: true, drift: true },
        expect: Expect::Error("metadata drift"),
    },
    Case {
        name: CaseName("the measurement path runs a request without observations once"),
        word: Word::Succeeds,
        input: || Scenario::MeasuredPlain,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the measurement path refuses observation-bearing requests unexecuted"),
        word: Word::Refuses,
        input: || Scenario::ObservationBearing,
        expect: Expect::Error("verification required"),
    },
];

/// Names what an error means by its message, so rows state the refusal and not its wording.
fn category(message: &str) -> &'static str {
    if message.contains("expected-vs-candidate") {
        "candidate mismatch"
    } else if message.contains("expected-vs-reference") {
        "reference mismatch"
    } else if message == "operational failure" {
        "candidate startup"
    } else if message.contains("reference: operational failure") {
        "reference startup"
    } else if message.contains("defaults lack implementation or explicit deferral") {
        "unaccounted default"
    } else if message == "no executable rule cases" {
        "no cases"
    } else if message == "tool input coverage differs from requested corpus" {
        "input coverage"
    } else if message.contains("fixture has no independent discrepancy artifacts") {
        "unproven evidence"
    } else if message.contains("executable metadata drifted") {
        "metadata drift"
    } else if message.contains("require verification preparation") {
        "verification required"
    } else {
        "unexpected error"
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::OmittedInput => omitted_input(),
        Scenario::ReferenceMultiplicity => reference_multiplicity(),
        Scenario::ReferenceFrontend => reference_frontend(),
        Scenario::ReferenceWrongEnd => reference_wrong_end(),
        Scenario::ReferenceMiscounted { expected_count } => reference_miscounted(expected_count),
        Scenario::ReferenceNoCases => reference_no_cases(),
        Scenario::ReferenceStartup => reference_startup(),
        Scenario::NativeIntended => native_intended(),
        Scenario::NativeWrongOracle => native_wrong_oracle(),
        Scenario::NativeReference => native_reference(),
        Scenario::NativeWrongReference => native_wrong_reference(),
        Scenario::NativeStaleEvidence => native_stale_evidence(),
        Scenario::ModeDrift { expected, drift } => mode_drift(expected, drift),
        Scenario::MeasuredPlain => measured_plain(),
        Scenario::ObservationBearing => observation_bearing(),
        Scenario::Agreement
        | Scenario::MissingCandidate
        | Scenario::MissingReference
        | Scenario::NoCases
        | Scenario::StartupFailure
        | Scenario::FullAgreement
        | Scenario::MissingFrontend
        | Scenario::RuleProjection => shard(scenario),
    }
    .map_err(|error| category(&error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn omitted_input() -> Result<Value, Error> {
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: None,
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        staged_sources: None,
        file_metadata: None,
        context_manifest: None,
        root: ".".into(),
        state: "cache".into(),
        files: vec![ryo::linter::RelativePath("input.py".into())],
    };
    super::service::run(&Empty, &Empty, &request).map(|_| 0)
}

/// The paired verification of the prepared case, which executes both contestants.
fn shard(scenario: Scenario) -> Result<Value, Error> {
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
    let result = super::service::verify_prepared_shard(
        &suite,
        &reference,
        &candidate,
        &execution,
        &crate::outbound::inventory::factory::local(),
        Path::new("rules"),
        suite.prepared.clone(),
        CaseSelection::default(),
    );
    if matches!(scenario, Scenario::StartupFailure) {
        assert_eq!(history_len(&execution), Some(1));
    } else if !matches!(scenario, Scenario::NoCases) {
        assert_eq!(history_len(&execution), Some(2));
    } else {
        assert_eq!(
            history_len(&execution),
            Some(0),
            "no cases must launch no contestants"
        );
    }
    result.map(|count| count.0)
}

/// The reference-only verification of `suite` with a reference answering `reference`.
fn reference_only(
    suite: &SuiteFake,
    reference: &LinterFake,
    execution: &ExecutionFake,
    rules: &str,
) -> Result<Value, Error> {
    super::service::verify_prepared_reference_cases(
        suite,
        reference,
        execution,
        &crate::outbound::inventory::factory::local(),
        Path::new(rules),
        suite.prepared.clone(),
    )
    .map(|count| count.0)
}

fn succeeding() -> ExecutionFake {
    ExecutionFake {
        status: 0,
        ..Default::default()
    }
}

/// The first prepared case, compared as a full report.
fn full_case(suite: &mut SuiteFake) -> &mut PreparedCase {
    let case = suite.prepared.first_mut().expect("fixture setup must include its case");
    case.comparison = CaseComparison::Full;
    case
}

fn reference_multiplicity() -> Result<Value, Error> {
    let (mut suite, finding) = prepared(Scenario::Agreement);
    full_case(&mut suite).expected.diagnostics = vec![finding.clone(), finding.clone()];
    let reference = LinterFake {
        diagnostics: vec![finding.clone(), finding],
        ..Default::default()
    };
    let execution = succeeding();
    let result = reference_only(&suite, &reference, &execution, "rules");
    assert_eq!(history_len(&execution), Some(1));
    result
}

fn reference_frontend() -> Result<Value, Error> {
    let (mut suite, _) = prepared(Scenario::RuleProjection);
    let mut reference = LinterFake {
        diagnostics: vec![
            BenchmarkFinding {
                rule: "E722".try_into().unwrap(),
                file: ryo::linter::RelativePath("case.py".into()),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(6),
            },
            BenchmarkFinding {
                rule: "F401".try_into().unwrap(),
                file: ryo::linter::RelativePath("case.py".into()),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(20),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(24),
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
    let execution = succeeding();
    let result = reference_only(&suite, &reference, &execution, "rules");
    assert_eq!(history_len(&execution), Some(1));
    result
}

fn reference_wrong_end() -> Result<Value, Error> {
    let (mut suite, finding) = prepared(Scenario::Agreement);
    full_case(&mut suite)
        .expected
        .diagnostics
        .first_mut()
        .expect("fixture setup must include its finding")
        .end = ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(7);
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    reference_only(&suite, &reference, &succeeding(), "rules")
}

/// An authored case with `expected_count` copies of the finding, against a reference reporting it once.
fn reference_miscounted(expected_count: usize) -> Result<Value, Error> {
    let (mut suite, finding) = prepared(Scenario::Agreement);
    full_case(&mut suite).expected.diagnostics = vec![finding.clone(); expected_count];
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    reference_only(&suite, &reference, &succeeding(), "rules")
}

fn reference_no_cases() -> Result<Value, Error> {
    let (empty, _) = prepared(Scenario::NoCases);
    let execution = succeeding();
    let result = reference_only(&empty, &LinterFake::default(), &execution, "rules");
    assert_eq!(history_len(&execution), Some(0));
    result
}

fn reference_startup() -> Result<Value, Error> {
    let (suite, finding) = prepared(Scenario::Agreement);
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    let execution = ExecutionFake {
        status: 2,
        ..Default::default()
    };
    let result = reference_only(&suite, &reference, &execution, "rules");
    assert_eq!(history_len(&execution), Some(1));
    result
}

fn history_len(execution: &ExecutionFake) -> Option<usize> {
    execution.history.lock().ok().map(|history| history.len())
}

/// Paired verification of the discrepancy fixture against the plain reference and `intended`.
fn native_pair(
    suite: &SuiteFake,
    intended: &CoveredCandidate,
    execution: &ExecutionFake,
) -> Result<Value, Error> {
    super::service::verify_prepared_shard(
        suite,
        &LinterFake::default(),
        intended,
        execution,
        &crate::outbound::inventory::factory::local(),
        Path::new("/profile/rules"),
        suite.prepared.clone(),
        CaseSelection::default(),
    )
    .map(|count| count.0)
}

fn native_intended() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    let intended = CoveredCandidate(LinterFake {
        diagnostics: suite.prepared[0].expected.diagnostics.clone(),
        ..Default::default()
    });
    let execution = succeeding();
    let result = native_pair(&suite, &intended, &execution);
    assert_eq!(history_len(&execution), Some(2), "both actual sides executed");
    let mut canonical = suite.prepared.clone();
    crate::application::discrepancy_attachment::service::attach(&suite, Path::new("/profile/rules"), &mut canonical)
        .expect("application owns wire conversion");
    assert!(
        crate::domain::discrepancy::service::matches_intended(
            &canonical[0],
            crate::domain::discrepancy::service::reference_expected(&canonical[0])
                .expect("diagnostic expectation")
        ) == crate::domain::discrepancy::types::IntentMatch::Differs,
        "JSONL retains raw reference mismatch"
    );
    result
}

fn native_wrong_oracle() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    let wrong_oracle = CoveredCandidate(LinterFake::default());
    native_pair(&suite, &wrong_oracle, &succeeding())
}

fn native_reference() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    reference_only(
        &suite,
        &LinterFake::default(),
        &succeeding(),
        "/profile/rules",
    )
}

fn native_wrong_reference() -> Result<Value, Error> {
    let suite = discrepancy_suite();
    let wrong_reference = LinterFake {
        diagnostics: suite.prepared[0].expected.diagnostics.clone(),
        ..Default::default()
    };
    reference_only(&suite, &wrong_reference, &succeeding(), "/profile/rules")
}

fn native_stale_evidence() -> Result<Value, Error> {
    let mut stale = discrepancy_suite();
    stale.discrepancy_artifacts_valid = false;
    reference_only(&stale, &LinterFake::default(), &succeeding(), "/profile/rules")
}

/// Sets the permission bits of `path`; a file system with no modes has none to set.
#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    std::fs::set_permissions(
        path,
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(mode),
    )
    .unwrap();
}
#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

/// A reference that, on its second preparation, changes the mode of the case file.
struct DriftingReference {
    calls: std::cell::Cell<usize>,
    expected: bool,
    drift: bool,
}
impl LinterPort for DriftingReference {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(
        &self,
        request: &RunRequest,
    ) -> Result<ryo_benchmark_contracts::domain::measurements::types::Invocation, Error> {
        let calls = self.calls.get() + 1;
        self.calls.set(calls);
        if calls == 2 && self.drift {
            set_mode(
                &request.root.join("case.py"),
                if self.expected { 0o644 } else { 0o744 },
            );
        }
        LinterFake::default().prepare(request)
    }
    fn decode(
        &self,
        request: &RunRequest,
        output: ryo_benchmark_contracts::domain::measurements::types::ProcessOutput,
    ) -> Result<RunReport, Error> {
        LinterFake::default().decode(request, output)
    }
}

fn mode_drift(expected: bool, drift: bool) -> Result<Value, Error> {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("case.py"), "same bytes").unwrap();
    set_mode(
        &root.path().join("case.py"),
        if expected { 0o744 } else { 0o644 },
    );
    let (mut suite, _) = prepared(Scenario::Agreement);
    let profile = suite.profile.as_mut().unwrap();
    profile.defaults.truncate(1);
    profile.file_observation =
        Some(ryo_benchmark_contracts::domain::workloads::types::FileObservation::PosixExecutable);
    let case = &mut suite.prepared[0];
    case.name = "first".into();
    case.expected.diagnostics.clear();
    case.request.root = root.path().to_path_buf();
    case.request.file_metadata = Some(std::collections::BTreeMap::from([(
        case.request.files[0].clone(),
        expected,
    )]));
    let mut second = case.clone();
    second.name = "second".into();
    suite.prepared.push(second);
    let reference = DriftingReference {
        calls: std::cell::Cell::new(0),
        expected,
        drift,
    };
    let execution = succeeding();
    let result = super::service::verify_prepared_shard(
        &suite,
        &reference,
        &LinterFake::default(),
        &execution,
        &crate::outbound::inventory::factory::local(),
        Path::new("rules"),
        suite.prepared.clone(),
        CaseSelection::default(),
    );
    assert_eq!(reference.calls.get(), 2);
    if drift {
        assert_eq!(
            history_len(&execution),
            Some(3),
            "second candidate must not execute after reference changes mode"
        );
    } else {
        assert_eq!(history_len(&execution), Some(4));
    }
    result.map(|count| count.0)
}

fn measured_plain() -> Result<Value, Error> {
    let (suite, _) = prepared(Scenario::Agreement);
    let execution = succeeding();
    super::service::run(
        &LinterFake::default(),
        &execution,
        &suite.prepared[0].request,
    )?;
    Ok(history_len(&execution).unwrap_or_default())
}

fn observation_bearing() -> Result<Value, Error> {
    let (suite, _) = prepared(Scenario::Agreement);
    let mut request = suite.prepared[0].request.clone();
    request.file_metadata = Some(std::collections::BTreeMap::from([(
        request.files[0].clone(),
        false,
    )]));
    let execution = succeeding();
    let result = super::service::run(&LinterFake::default(), &execution, &request);
    assert_eq!(
        history_len(&execution),
        Some(0),
        "unsupported observation claims must not prepare or execute a measured contestant"
    );
    result.map(|_| 0)
}

use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
struct Empty;
impl LinterPort for Empty {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
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
            execution_metadata:
                ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
            project_context: None,
            skipped: Vec::new(),
            coverage: Some(
                ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                    selected: 1,
                    evaluated: 1,
                    skipped: 0,
                    unknown_or_unread: 0,
                },
            ),
            diagnostics: vec![],
            files: vec![],
        })
    }
}
impl ExecutionPort for Empty {
    fn profile(&self) -> ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile {
        ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile::Wall
    }
    fn run(&self, _: Invocation) -> Result<ProcessOutput, Error> {
        Ok(ProcessOutput {
            signal: None,
            status: ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode(0),
            stdout: ryo_benchmark_contracts::domain::measurements::types::CapturedBytes(vec![]),
            stderr: ryo_benchmark_contracts::domain::measurements::types::CapturedBytes(vec![]),
            wall: std::time::Duration::ZERO,
            resources:
                ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly,
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
            start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(9),
            end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(19),
        };
        reference.diagnostics.push(frontend.clone());
        if !matches!(scenario, Scenario::MissingFrontend) {
            candidate.diagnostics.push(frontend.clone());
        }
        if matches!(scenario, Scenario::RuleProjection) {
            let unrelated = BenchmarkFinding {
                rule: "F401".try_into().expect("unrelated lint identity"),
                file: ryo::linter::RelativePath("case.py".into()),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(20),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(24),
            };
            reference.diagnostics.push(unrelated.clone());
            candidate.diagnostics.push(unrelated);
        }
        if !matches!(scenario, Scenario::RuleProjection) {
            suite
                .prepared
                .first_mut()
                .expect("authored case")
                .comparison =
                ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full;
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
        start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0),
        end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(6),
    };
    let mut suite = SuiteFake::default();
    if !matches!(scenario, Scenario::NoCases) {
        suite.prepared.push(PreparedCase {
            reference_discrepancy: None,
            comparison: ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Rule,
            rule: ryo::linter::RuleId::try_from("E722").unwrap(),
            name: "positive".into(),
            request: RunRequest {
                performance: Default::default(),
                selected_rules: None,
                rule_observations: None,
                rule_context: None,
                project_search_roots: None,
                given_project_inventory: Default::default(),
                staged_sources: None,
                file_metadata: None,
                context_manifest: None,
                root: ".".into(),
                files: vec![ryo::linter::RelativePath("case.py".into())],
                state: "cache".into(),
            },
            expected: RunReport {
                execution_metadata:
                    ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                project_context: None,
                files: vec![ryo::linter::RelativePath("case.py".into())],
                skipped: Vec::new(),
                coverage: Some(
                    ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                        selected: 1,
                        evaluated: 1,
                        skipped: 0,
                        unknown_or_unread: 0,
                    },
                ),
                diagnostics: vec![expected.clone()],
            },
        });
    }
    suite.profile = Some(ryo_benchmark_contracts::domain::workloads::types::Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: vec!["E722".try_into().unwrap(), "F901".try_into().unwrap()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        project_search_roots: None,
    });
    (suite, expected)
}

/// Candidate fixture exposes complete execution accounting independently of its findings.
struct CoveredCandidate(LinterFake);
impl LinterPort for CoveredCandidate {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        LinterPort::prepare(&self.0, request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        let mut report = LinterPort::decode(&self.0, request, output)?;
        report.coverage = Some(
            ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                selected: 1,
                evaluated: 1,
                skipped: 0,
                unknown_or_unread: 0,
            },
        );
        Ok(report)
    }
}

/// Explicit fake-port evidence lets orchestration tests distinguish reference and candidate contracts.
fn discrepancy_suite() -> SuiteFake {
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
    use ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceDiscrepancy;
    use ryo_benchmark_contracts::domain::workloads::types::ReferenceDiscrepancyIdentity;
    use ryo_benchmark_contracts::domain::workloads::types::Tool;
    use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence;
    use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidenceKind;
    let (mut suite, _) = prepared(Scenario::Agreement);
    let case = suite.prepared.first_mut().expect("authored case");
    case.comparison = ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full;
    case.request.root = "/case".into();
    let mut reference = case.expected.clone();
    reference.diagnostics.clear();
    let profile_hash = ArtifactDigest("a".repeat(64));
    let binary_hash = ArtifactDigest("b".repeat(64));
    let profile = Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
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
        source_language: None,
        project_search_roots: None,
    };
    let invocation = LinterFake::default()
        .prepare(&case.request)
        .expect("fixture invocation");
    let settings = crate::domain::canonical_text::service::text_digest(
        &crate::application::discrepancy_validation::service::settings_text(
            &invocation,
            case,
            &profile,
        )
        .expect("fixture settings"),
    );
    case.reference_discrepancy = Some(PreparedReferenceDiscrepancy {
        basis: ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect,
        identity: ReferenceDiscrepancyIdentity {
            file_metadata_sha256: None,
            tool: "fixture".into(),
            version: "1".into(),
            observed_executable_sha256: binary_hash.clone(),
            profile_sha256: profile_hash.clone(),
            settings_sha256: settings,
            source_sha256: ArtifactDigest("c".repeat(64)),
            intended_output_sha256: crate::domain::discrepancy::service::output_digest(
                &case.expected,
            ),
            reference_output_sha256: crate::domain::discrepancy::service::output_digest(&reference),
        },
        expected:
            ryo_benchmark_contracts::domain::process::types::ReferenceExpectation::Diagnostics(
                reference,
            ),
        reason: "independent language contract differs from the pinned reference".into(),
        evidence: vec![DiscrepancyEvidence::Language {
            path: "evidence.json".into(),
            sha256: ArtifactDigest("d".repeat(64)),
            kind: DiscrepancyEvidenceKind::IndependentLanguageRun,
            language_version: "3.10".into(),
            target_configuration: "python-target=3.10".into(),
            version_rationale: "exact target independently evaluated".into(),
        }],
    });
    suite.profile = Some(profile);
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
            basis: crate::outbound::suite::types::discrepancy::DiscrepancyBasisWire::LanguageDefect,
            discrepancy,
        },
    );
    suite
}
