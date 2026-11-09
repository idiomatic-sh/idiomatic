//! Invocation, operational failures and exact-source decoding through the actual adapter.
use super::engine_test::boundary_failure_test::{BoundaryFailure, failure};
use super::{adapter::LinterRuffAdapter, constants, settings::LinterRuffSettings};
use ryo::linter::RuleId;
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::outbound::linter::fixtures::contract::check;
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

#[path = "engine_test/adapter_scenario_test.rs"]
mod adapter_scenario_test;
use adapter_scenario_test::{
    AdapterScenario, finding_bytes, invocation_matches,
    reference_roots_must_match_the_declared_project_scope,
};

type Input = AdapterScenario;
type Value = bool;
type Failure = BoundaryFailure;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("canonical root and ordinary diagnostic path identify the same file"),
        word: Word::Succeeds,
        input: || AdapterScenario::CanonicalRoot,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("concrete_ruff_answers_shared_linter_contract"),
        word: Word::Succeeds,
        input: || AdapterScenario::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("isolated_defaults_and_one_worker_invocation"),
        word: Word::Succeeds,
        input: || AdapterScenario::Invocation,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("python_311_target_is_passed_to_the_reference_cli"),
        word: Word::Succeeds,
        input: || AdapterScenario::InvocationPy311,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("lint_json_with_warning_is_valid"),
        word: Word::Succeeds,
        input: || AdapterScenario::WarningFinding,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("one selected rule keeps the Python 3.10 frontend diagnostic"),
        word: Word::Succeeds,
        input: || AdapterScenario::FrontendFinding,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("clean_json_with_warning_is_valid"),
        word: Word::Succeeds,
        input: || AdapterScenario::CleanWarning,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operational_error_is_not_a_finding"),
        word: Word::Refuses,
        input: || AdapterScenario::Operational,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("empty_files_cannot_walk"),
        word: Word::Refuses,
        input: || AdapterScenario::EmptyInput,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("contradictory_exit_status_is_refused"),
        word: Word::Refuses,
        input: || AdapterScenario::ContradictoryStatus,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("unrequested_file_is_refused"),
        word: Word::Refuses,
        input: || AdapterScenario::UnrequestedFile,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("duplicate_requested_file_is_refused"),
        word: Word::Refuses,
        input: || AdapterScenario::DuplicateInput,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("unexpected_rule_is_refused"),
        word: Word::Refuses,
        input: || AdapterScenario::UnexpectedRule,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("reversed_span_is_refused"),
        word: Word::Refuses,
        input: || AdapterScenario::ReversedSpan,
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("discovery_failure_is_not_empty_success"),
        word: Word::Fails,
        input: || AdapterScenario::MissingExecutable,
        expect: Expect::Error(BoundaryFailure::Io(std::io::ErrorKind::NotFound)),
    },
    Case {
        name: CaseName("missing_source_with_zero_findings_is_not_checked"),
        word: Word::Fails,
        input: || AdapterScenario::MissingCleanFile,
        expect: Expect::Error(BoundaryFailure::Io(std::io::ErrorKind::NotFound)),
    },
    Case {
        name: CaseName("invalid_json_is_not_clean"),
        word: Word::Fails,
        input: || AdapterScenario::InvalidJson,
        expect: Expect::Error(BoundaryFailure::Json),
    },
    Case {
        name: CaseName("reference_roots_must_match_the_declared_project_scope"),
        word: Word::Succeeds,
        input: || AdapterScenario::ReferenceRootsMustMatchTheDeclaredProjectScope,
        expect: Expect::Value(true),
    },
];

#[expect(
    clippy::expect_used,
    reason = "fixed fixture rule identifiers must be valid before testing the adapter"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one scenario dispatcher keeps the adapter contract matrix aligned with its cases"
)]
fn exercise(scenario: Input) -> Result<Value, Error> {
    if matches!(
        scenario,
        AdapterScenario::ReferenceRootsMustMatchTheDeclaredProjectScope
    ) {
        return reference_roots_must_match_the_declared_project_scope().map(|()| true);
    }
    let root = tempfile::tempdir()?;
    let executable = root.path().join("missing-ruff");
    let target_version = if matches!(scenario, AdapterScenario::InvocationPy311) {
        "py311"
    } else {
        "py310"
    };
    let ruff = LinterRuffAdapter::new(LinterRuffSettings {
        executable: executable.clone(),
        defaults: [RuleId::try_from("E722").expect("valid rule")]
            .into_iter()
            .collect(),
        target_version: target_version.into(),
    });
    if matches!(scenario, AdapterScenario::Contract) {
        check(&ruff);
        return Ok(true);
    }
    let mut request = RunRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata:
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.path().to_owned(),
        files: vec![ryo::linter::RelativePath("case.py".into())],
        state: root.path().join("cache"),
    };
    if matches!(scenario, AdapterScenario::MissingExecutable) {
        return ryo_benchmark::outbound::discovery::interface::DiscoveryPort::files(
            &ryo_benchmark::outbound::discovery::factory::listing(
                executable.clone(),
                constants::DISCOVERY_ARGUMENTS
                    .into_iter()
                    .map(std::ffi::OsString::from)
                    .collect(),
            ),
            root.path(),
        )
        .map(|_| false);
    }
    if matches!(scenario, AdapterScenario::EmptyInput) {
        request.files.clear();
        return ruff.prepare(&request).map(|_| false);
    }
    if matches!(
        scenario,
        AdapterScenario::Invocation | AdapterScenario::InvocationPy311
    ) {
        return invocation_matches(&ruff, &request, &executable, target_version);
    }
    if !matches!(
        scenario,
        AdapterScenario::MissingCleanFile | AdapterScenario::Operational
    ) {
        std::fs::write(
            root.path().join("case.py"),
            "try:\n    pass\nexcept: # ruff: noqa\n    pass\n",
        )?;
    }
    if matches!(scenario, AdapterScenario::DuplicateInput) {
        request
            .files
            .push(ryo::linter::RelativePath("case.py".into()));
    }
    let filename = root
        .path()
        .join(if matches!(scenario, AdapterScenario::UnrequestedFile) {
            "other.py"
        } else {
            "case.py"
        });
    let filename = if matches!(scenario, AdapterScenario::CanonicalRoot) {
        request.root = root.path().canonicalize()?;
        dunce::simplified(&filename.canonicalize()?).to_owned()
    } else {
        filename
    };
    let finding = finding_bytes(scenario, &filename)?;
    let (status, stdout) = match scenario {
        AdapterScenario::Operational => (2_i32, b"[]".to_vec()),
        AdapterScenario::MissingCleanFile | AdapterScenario::CleanWarning => {
            (0_i32, b"[]".to_vec())
        }
        AdapterScenario::InvalidJson => (0_i32, b"not json".to_vec()),
        AdapterScenario::ContradictoryStatus => (0_i32, finding),
        AdapterScenario::Contract
        | AdapterScenario::EmptyInput
        | AdapterScenario::Invocation
        | AdapterScenario::InvocationPy311
        | AdapterScenario::MissingExecutable
        | AdapterScenario::WarningFinding
        | AdapterScenario::FrontendFinding
        | AdapterScenario::UnrequestedFile
        | AdapterScenario::DuplicateInput
        | AdapterScenario::UnexpectedRule
        | AdapterScenario::ReversedSpan
        | AdapterScenario::CanonicalRoot
        | AdapterScenario::ReferenceRootsMustMatchTheDeclaredProjectScope => (1_i32, finding),
    };
    let report = ruff.decode(
        &request,
        ProcessOutput {
            signal: ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Exited,
            status: ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode(status),
            stdout: ryo_benchmark_contracts::domain::measurements::types::CapturedBytes(stdout),
            stderr: ryo_benchmark_contracts::domain::measurements::types::CapturedBytes(
                b"warning: file-level noqa must be on its own line\n".to_vec(),
            ),
            wall: std::time::Duration::default(),
            resources:
                ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly,
        },
    )?;
    let expected = if matches!(scenario, AdapterScenario::CleanWarning) {
        vec![]
    } else if matches!(scenario, AdapterScenario::FrontendFinding) {
        vec![
            BenchmarkFinding {
                rule: RuleId::try_from("E722").expect("fixture rule"),
                file: ryo::linter::RelativePath("case.py".into()),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(14),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(20),
            },
            BenchmarkFinding {
                rule: RuleId::try_from("invalid-syntax").expect("frontend rule"),
                file: ryo::linter::RelativePath("case.py".into()),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(20),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(21),
            },
        ]
    } else {
        vec![BenchmarkFinding {
            rule: RuleId::try_from("E722").expect("fixture rule"),
            file: ryo::linter::RelativePath("case.py".into()),
            start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(14),
            end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(20),
        }]
    };
    Ok(report.files == request.files
        && report.diagnostics == expected
        && matches!(
            report.execution_metadata,
            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Unavailable { .. }
        )
        && report.coverage
            == ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Unaccounted)
}
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input).map_err(failure)
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
