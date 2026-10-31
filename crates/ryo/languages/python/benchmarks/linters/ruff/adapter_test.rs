//! Invocation, operational failures and exact-source decoding through the actual adapter.
use super::{BoundaryFailure, adapter::LinterRuffAdapter, failure, settings::LinterRuffSettings};
use ryo::linter::RuleId;
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::BenchmarkFinding},
    measurements::types::ProcessOutput,
    suite::types::RunRequest,
};
use ryo_benchmark::outbound::linter::fixtures::contract::check;
use ryo_benchmark::outbound::linter::interface::LinterPort as _;

#[derive(Clone, Copy)]
enum AdapterScenario {
    Contract,
    Operational,
    EmptyInput,
    Invocation,
    MissingExecutable,
    WarningFinding,
    MissingCleanFile,
    CleanWarning,
    InvalidJson,
    ContradictoryStatus,
    UnrequestedFile,
    DuplicateInput,
    UnexpectedRule,
    ReversedSpan,
    CanonicalRoot,
}
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
        name: CaseName("lint_json_with_warning_is_valid"),
        word: Word::Succeeds,
        input: || AdapterScenario::WarningFinding,
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
];

#[expect(
    clippy::expect_used,
    reason = "fixed fixture rule identifiers must be valid before testing the adapter"
)]
fn exercise(scenario: Input) -> Result<Value, Error> {
    let root = tempfile::tempdir()?;
    let executable = root.path().join("missing-ruff");
    let ruff = LinterRuffAdapter::new(LinterRuffSettings {
        executable: executable.clone(),
        defaults: [RuleId::try_from("E722").expect("valid rule")]
            .into_iter()
            .collect(),
    });
    if matches!(scenario, AdapterScenario::Contract) {
        check(&ruff);
        return Ok(true);
    }
    let mut request = RunRequest {
        root: root.path().to_owned(),
        files: vec![ryo::linter::RelativePath("case.py".into())],
        state: root.path().join("cache"),
    };
    if matches!(scenario, AdapterScenario::MissingExecutable) {
        return ruff.files(root.path()).map(|_| false);
    }
    if matches!(scenario, AdapterScenario::EmptyInput) {
        request.files.clear();
        return ruff.prepare(&request).map(|_| false);
    }
    if matches!(scenario, AdapterScenario::Invocation) {
        return invocation_matches(&ruff, &request, &executable);
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
    let code = if matches!(scenario, AdapterScenario::UnexpectedRule) {
        "F401"
    } else {
        "E722"
    };
    let end = if matches!(scenario, AdapterScenario::ReversedSpan) {
        1_usize
    } else {
        7_usize
    };
    let start = if matches!(scenario, AdapterScenario::ReversedSpan) {
        7_usize
    } else {
        1_usize
    };
    let finding = serde_json::to_vec(
        &serde_json::json!([{"code":code,"filename":filename,"cell":null,"location":{"row":3_usize,"column":start},"end_location":{"row":3_usize,"column":end}}]),
    ).map_err(|error| Error::Json(Box::new(error)))?;
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
        | AdapterScenario::MissingExecutable
        | AdapterScenario::WarningFinding
        | AdapterScenario::UnrequestedFile
        | AdapterScenario::DuplicateInput
        | AdapterScenario::UnexpectedRule
        | AdapterScenario::ReversedSpan
        | AdapterScenario::CanonicalRoot => (1_i32, finding),
    };
    let report = ruff.decode(
        &request,
        ProcessOutput {
            status: ryo_benchmark::domain::measurements::types::ProcessExitCode(status),
            stdout: ryo_benchmark::domain::measurements::types::CapturedBytes(stdout),
            stderr: ryo_benchmark::domain::measurements::types::CapturedBytes(
                b"warning: file-level noqa must be on its own line\n".to_vec(),
            ),
            wall: std::time::Duration::default(),
            resources: ryo_benchmark::domain::measurements::types::ResourceObservation::WallOnly,
        },
    )?;
    let expected = if matches!(scenario, AdapterScenario::CleanWarning) {
        vec![]
    } else {
        vec![BenchmarkFinding {
            rule: RuleId::try_from("E722").expect("fixture rule"),
            file: ryo::linter::RelativePath("case.py".into()),
            start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(14),
            end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(20),
        }]
    };
    Ok(report.files == request.files && report.diagnostics == expected)
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

fn invocation_matches(
    ruff: &LinterRuffAdapter,
    request: &RunRequest,
    executable: &std::path::Path,
) -> Result<bool, Error> {
    let invocation = ruff.prepare(request)?;
    let expected: Vec<std::ffi::OsString> = [
        "check",
        "--isolated",
        "--output-format",
        "json",
        "--cache-dir",
    ]
    .into_iter()
    .map(Into::into)
    .chain([
        request.state.join("ruff").into_os_string(),
        "--select".into(),
        "E722".into(),
        "--".into(),
        "case.py".into(),
    ])
    .collect();
    Ok(invocation.arguments == expected
        && invocation
            .environment
            .get(std::ffi::OsStr::new("RAYON_NUM_THREADS"))
            == Some(&"1".into())
        && invocation.directory == request.root
        && invocation.executable == executable)
}
