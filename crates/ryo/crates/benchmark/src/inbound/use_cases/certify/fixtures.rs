//! Executable certification scenarios shared by unit and acceptance proofs.
#![expect(
    clippy::expect_used,
    reason = "fixture setup and fake state are test preconditions; a panic fails the unit or acceptance proof that calls this module"
)]
use super::constants;
use crate::domain::{
    diagnostics::{
        errors::DiagnosticsError as Error,
        types::{BenchmarkFinding, RunReport},
    },
    measurements::types::{
        CollectorDigest, CollectorProfile, ExecutionProfile, Invocation, ProcessOutput, RunOrder,
    },
    suite::types::{BenchmarkCoverage, RunRequest},
};
use crate::outbound::linter::interface::LinterPort;
use sha2::{Digest as _, Sha256};
use std::path::PathBuf;
#[derive(Clone, Copy, Debug)]
/// The independently controlled mutation applied after case certification.
pub enum Change {
    /// Keep every certified input unchanged.
    None,
    /// Change the frozen edit workload.
    Patch,
    /// Change the frozen project identity.
    Manifest,
    /// Change the resource collector sealed inside the workload.
    Collector,
    /// Change the collection mode without changing any certified file.
    Instrumentation,
    /// Change the collector's configured pin without changing its bytes.
    CollectorPin,
    /// Change the collector's configured path without changing its original bytes.
    CollectorPath,
    /// Change the authored query bytes.
    Query,
    /// Change the contestant executable bytes.
    Executable,
    /// Change the prepared command profile.
    Command,
}
#[derive(Clone, Debug)]
/// Deterministic contestant whose findings depend only on the fixture source.
struct Contestant {
    /// Executable identity used by preparation and certificate checks.
    executable: PathBuf,
    /// Mutable arguments used to prove command-profile drift is refused.
    arguments: Vec<std::ffi::OsString>,
}
impl LinterPort for Contestant {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments: self.arguments.clone(),
            environment: std::collections::BTreeMap::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, _: ProcessOutput) -> Result<RunReport, Error> {
        let diagnostics = request
            .files
            .iter()
            .filter_map(|file| {
                let source = std::fs::read_to_string(request.root.join(file.0.as_ref()))
                    .expect(constants::FIXTURE_PRECONDITION);
                (source == constants::FIXTURE_POSITIVE_SOURCE).then(|| BenchmarkFinding {
                    rule: ryo::linter::RuleId::try_from(constants::FIXTURE_RULE_ID)
                        .expect(constants::FIXTURE_PRECONDITION),
                    file: file.clone(),
                    start: crate::domain::diagnostics::types::SourceByteOffset(0),
                    end: crate::domain::diagnostics::types::SourceByteOffset(1),
                })
            })
            .collect();
        Ok(RunReport {
            execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata::Complete,
            project_context: None,
            skipped: Vec::new(),
            coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
                selected: request.files.len(),
                evaluated: request.files.len(),
                skipped: 0,
                unknown_or_unread: 0,
            }),
            files: request.files.clone(),
            diagnostics,
        })
    }
}
#[derive(Debug)]
/// Records launches while allowing independent collection-profile mutation.
struct Observations(
    crate::outbound::execution::fixtures::fake::ExecutionFake,
    std::sync::Mutex<ExecutionProfile>,
);
impl crate::outbound::execution::interface::ExecutionPort for Observations {
    fn profile(&self) -> ExecutionProfile {
        self.1
            .lock()
            .expect(constants::FIXTURE_PRECONDITION)
            .clone()
    }
    fn run(&self, mut invocation: Invocation) -> Result<ProcessOutput, Error> {
        invocation.executable = constants::FIXTURE_FAKE_PROGRAM.into();
        let mut output =
            crate::outbound::execution::interface::ExecutionPort::run(&self.0, invocation)?;
        output.resources = crate::domain::measurements::types::ResourceObservation::Collected(
            crate::domain::measurements::types::Resources {
                cpu: std::time::Duration::from_millis(1),
                peak_rss_bytes: crate::domain::measurements::types::ResidentBytes(4096),
            },
        );
        Ok(output)
    }
}
/// Execute the public certificate and measurement operations with a controlled mutation.
///
/// # Errors
/// Answers unit refusal only after asserting the precise diagnostic and absence of launches.
/// # Panics
/// Panics on fixture setup failure or an unexpected observation.
#[expect(
    clippy::result_unit_err,
    reason = "the scenario asserts the complete refusal before returning its case-table marker"
)]
#[expect(
    clippy::too_many_lines,
    reason = "this single certification timeline preserves the before/after mutation and launch assertions together"
)]
#[expect(
    clippy::clone_on_ref_ptr,
    reason = "clones coerce concrete fake Arcs into independently injected trait-object ports"
)]
pub fn exercise(
    change: Change,
    constructor: impl FnOnce(
        std::sync::Arc<dyn crate::outbound::suite::interface::SuitePort>,
        std::sync::Arc<dyn LinterPort>,
        std::sync::Arc<dyn LinterPort>,
        std::sync::Arc<dyn crate::outbound::execution::interface::ExecutionPort>,
    ) -> super::use_case::CertifyUseCaseImpl,
) -> Result<crate::domain::suite::types::CaseCount, ()> {
    let root = tempfile::tempdir().expect(constants::FIXTURE_PRECONDITION);
    let directory = root.path().join(constants::FIXTURE_PROFILE_DIRECTORY);
    let rules = directory.join(constants::FIXTURE_RULE_DIRECTORY_NAME);
    std::fs::create_dir_all(rules.join(constants::FIXTURE_RULE_ID))
        .expect(constants::FIXTURE_PRECONDITION);
    let executable = root.path().join(constants::FIXTURE_TOOL_FILE);
    let bytes = constants::FIXTURE_TOOL_SCRIPT.as_bytes();
    std::fs::write(&executable, bytes).expect(constants::FIXTURE_PRECONDITION);
    executable_permissions(&executable);
    std::fs::write(
        directory.join(constants::FIXTURE_PROFILE_FILE),
        format!(
            "tool = 'fixture'\nversion = '1'\nsha256 = ['{:x}']\ndefaults = ['A']\n",
            Sha256::digest(bytes)
        ),
    )
    .expect(constants::FIXTURE_PRECONDITION);
    std::fs::write(
        rules.join(constants::FIXTURE_QUERY_FILE),
        constants::FIXTURE_QUERY_CONTENT,
    )
    .expect(constants::FIXTURE_PRECONDITION);
    std::fs::write(
        rules.join(constants::FIXTURE_CASES_FILE),
        toml::to_string(&case_document()).expect(constants::FIXTURE_PRECONDITION),
    )
    .expect(constants::FIXTURE_PRECONDITION);
    let workload = root.path().join(constants::FIXTURE_WORKLOAD_DIRECTORY);
    std::fs::create_dir_all(&workload).expect(constants::FIXTURE_PRECONDITION);
    std::fs::write(
        workload.join(constants::FIXTURE_PROFILE_FILE),
        constants::FIXTURE_PROJECT_CONTENT,
    )
    .expect(constants::FIXTURE_PRECONDITION);
    std::fs::write(
        workload.join(constants::FIXTURE_PATCH_FILE),
        constants::FIXTURE_PATCH_CONTENT,
    )
    .expect(constants::FIXTURE_PRECONDITION);
    std::fs::write(
        workload.join(constants::FIXTURE_COLLECTOR_FILE),
        constants::FIXTURE_TOOL_SCRIPT,
    )
    .expect(constants::FIXTURE_PRECONDITION);
    let suite = std::sync::Arc::new(crate::outbound::suite::factory::local());
    let reference = std::sync::Arc::new(Contestant {
        executable: executable.clone(),
        arguments: vec![],
    });
    let mut candidate = std::sync::Arc::new(Contestant {
        executable: executable.clone(),
        arguments: vec![],
    });
    let execution = std::sync::Arc::new(Observations(
        crate::outbound::execution::fixtures::fake::ExecutionFake {
            status: 0,
            ..Default::default()
        },
        std::sync::Mutex::new(ExecutionProfile::Isolated(CollectorProfile {
            executable: workload.join(constants::FIXTURE_COLLECTOR_FILE),
            digest: CollectorDigest(format!(
                "{:x}",
                Sha256::digest(constants::FIXTURE_TOOL_SCRIPT.as_bytes())
            )),
        })),
    ));
    let operation = constructor(
        suite.clone(),
        reference.clone(),
        candidate.clone(),
        execution.clone(),
    );
    let certificate = super::interface::CertifyUseCase::certify(
        &operation,
        &directory,
        &workload,
        &executable,
        &executable,
        &rules,
        &root.path().join(constants::FIXTURE_CASE_DIRECTORY),
        BenchmarkCoverage::Accounted,
    )
    .expect(constants::FIXTURE_PRECONDITION);
    assert!(
        !certificate.fingerprint().0.is_empty(),
        "{}",
        constants::FIXTURE_DIGEST_REQUIRED
    );
    let measured_root = root.path().join(constants::FIXTURE_SOURCE_DIRECTORY);
    std::fs::create_dir_all(&measured_root).expect(constants::FIXTURE_PRECONDITION);
    std::fs::write(
        measured_root.join(constants::FIXTURE_SOURCE_FILE),
        constants::FIXTURE_POSITIVE_SOURCE,
    )
    .expect(constants::FIXTURE_PRECONDITION);
    let request = RunRequest {
        file_metadata: None,
        context_manifest: None,
        root: measured_root,
        files: vec![ryo::linter::RelativePath(
            constants::FIXTURE_SOURCE_FILE.into(),
        )],
        state: root.path().join(constants::FIXTURE_STATE_DIRECTORY),
    };
    let changed = match change {
        Change::None => None,
        Change::Patch => Some(workload.join(constants::FIXTURE_PATCH_FILE)),
        Change::Manifest => Some(workload.join(constants::FIXTURE_PROFILE_FILE)),
        Change::Collector => Some(workload.join(constants::FIXTURE_COLLECTOR_FILE)),
        Change::Instrumentation => {
            *execution.1.lock().expect(constants::FIXTURE_PRECONDITION) = ExecutionProfile::Wall;
            None
        }
        Change::CollectorPin | Change::CollectorPath => {
            if let ExecutionProfile::Isolated(profile) =
                &mut *execution.1.lock().expect(constants::FIXTURE_PRECONDITION)
            {
                if matches!(change, Change::CollectorPin) {
                    profile.digest.0 = constants::FIXTURE_ALTERED_BYTES.into();
                } else {
                    profile.executable = constants::FIXTURE_ALTERED_BYTES.into();
                }
            }
            None
        }
        Change::Query => Some(rules.join(constants::FIXTURE_QUERY_FILE)),
        Change::Executable => Some(executable),
        Change::Command => {
            std::sync::Arc::make_mut(&mut candidate)
                .arguments
                .push(constants::FIXTURE_ALTERED_ARGUMENT.into());
            None
        }
    };
    if let Some(path) = changed {
        std::fs::write(path, constants::FIXTURE_ALTERED_BYTES)
            .expect(constants::FIXTURE_PRECONDITION);
    }
    let prior = execution
        .0
        .history
        .lock()
        .expect(constants::FIXTURE_PRECONDITION)
        .len();
    let measurement = crate::inbound::use_cases::measure::use_case::MeasureUseCaseImpl::new(
        reference.clone(),
        candidate.clone(),
        execution.clone(),
        suite.clone(),
    );
    let result = crate::inbound::use_cases::measure::interface::MeasureUseCase::measure(
        &measurement,
        &certificate,
        &request,
        RunOrder::ReferenceFirst,
    );
    match result {
        Ok(_) => Ok(certificate.cases()),
        Err(error) => {
            assert_eq!(
                prior,
                execution
                    .0
                    .history
                    .lock()
                    .expect(constants::FIXTURE_PRECONDITION)
                    .len(),
                "{}",
                constants::FIXTURE_NO_EARLY_EXECUTION
            );
            let expected = match change {
                Change::Command => constants::FIXTURE_EXPECTED_COMMAND_REFUSAL,
                Change::Instrumentation | Change::CollectorPin | Change::CollectorPath => {
                    constants::FIXTURE_EXPECTED_INSTRUMENTATION_REFUSAL
                }
                Change::None
                | Change::Patch
                | Change::Manifest
                | Change::Collector
                | Change::Query
                | Change::Executable => constants::FIXTURE_EXPECTED_ARTIFACT_REFUSAL,
            };
            assert_eq!(
                error.to_string(),
                expected,
                "{}",
                constants::FIXTURE_REFUSAL_REQUIRED
            );
            Err(())
        }
    }
}

/// Encode independent positive and negative case records.
fn case_document() -> toml::Value {
    let record = |name: toml::Value, source: toml::Value, expected: Vec<toml::Value>| {
        toml::Value::Table(
            [
                (constants::FIXTURE_CASE_NAME.into(), name),
                (
                    constants::FIXTURE_CASE_FILES.into(),
                    toml::Value::Table(
                        [(constants::FIXTURE_SOURCE_FILE.into(), source)]
                            .into_iter()
                            .collect(),
                    ),
                ),
                (
                    constants::FIXTURE_CASE_EXPECTED.into(),
                    toml::Value::Array(expected),
                ),
            ]
            .into_iter()
            .collect(),
        )
    };
    let finding = toml::Value::Table(
        [
            (
                constants::FIXTURE_FINDING_FILE.into(),
                toml::Value::String(constants::FIXTURE_SOURCE_FILE.into()),
            ),
            (
                constants::FIXTURE_FINDING_START.into(),
                toml::Value::Integer(0),
            ),
            (
                constants::FIXTURE_FINDING_END.into(),
                toml::Value::Integer(1),
            ),
        ]
        .into_iter()
        .collect(),
    );
    toml::Value::Table(
        [(
            constants::FIXTURE_CASE_RECORDS.into(),
            toml::Value::Array(vec![
                record(
                    toml::Value::String(constants::FIXTURE_POSITIVE_NAME.into()),
                    toml::Value::String(constants::FIXTURE_POSITIVE_SOURCE.into()),
                    vec![finding],
                ),
                record(
                    toml::Value::String(constants::FIXTURE_NEGATIVE_NAME.into()),
                    toml::Value::String(constants::FIXTURE_NEGATIVE_SOURCE.into()),
                    vec![],
                ),
            ]),
        )]
        .into_iter()
        .collect(),
    )
}

/// Mark the fixed Unix fixture script executable before native tool verification.
#[cfg(unix)]
fn executable_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect(constants::FIXTURE_PRECONDITION);
}
/// Windows launches the fixed `.cmd` fixture without Unix executable mode bits.
#[cfg(not(unix))]
fn executable_permissions(_path: &std::path::Path) {}
