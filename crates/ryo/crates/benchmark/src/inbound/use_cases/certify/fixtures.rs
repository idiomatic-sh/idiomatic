//! Executable certification scenarios shared by unit and acceptance proofs.
use crate::domain::{
    diagnostics::{
        errors::DiagnosticsError as Error,
        types::{BenchmarkFinding, RunReport},
    },
    measurements::types::{Invocation, ProcessOutput, RunOrder},
    suite::types::{BenchmarkCoverage, RunRequest},
};
use crate::outbound::linter::interface::LinterPort;
use sha2::{Digest, Sha256};
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
struct Contestant {
    executable: PathBuf,
    arguments: Vec<std::ffi::OsString>,
}
impl LinterPort for Contestant {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments: self.arguments.clone(),
            environment: Default::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, _: ProcessOutput) -> Result<RunReport, Error> {
        let diagnostics = request
            .files
            .iter()
            .filter_map(|file| {
                let source = std::fs::read_to_string(request.root.join(file.0.as_ref())).unwrap();
                (source == super::constants::FIXTURE_POSITIVE_SOURCE).then(|| BenchmarkFinding {
                    rule: ryo::linter::RuleId::try_from(super::constants::FIXTURE_RULE_ID).unwrap(),
                    file: file.clone(),
                    start: crate::domain::diagnostics::types::SourceByteOffset(0),
                    end: crate::domain::diagnostics::types::SourceByteOffset(1),
                })
            })
            .collect();
        Ok(RunReport {
            files: request.files.clone(),
            diagnostics,
        })
    }
}
#[derive(Debug)]
struct Observations(
    crate::outbound::execution::fixtures::fake::ExecutionFake,
    std::sync::Mutex<crate::domain::measurements::types::ExecutionProfile>,
);
impl crate::outbound::execution::interface::ExecutionPort for Observations {
    fn profile(&self) -> crate::domain::measurements::types::ExecutionProfile {
        self.1.lock().unwrap().clone()
    }
    fn run(&self, mut invocation: Invocation) -> Result<ProcessOutput, Error> {
        invocation.executable = super::constants::FIXTURE_FAKE_PROGRAM.into();
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
#[cfg(unix)]
/// Execute the public certificate and measurement operations with a controlled mutation.
pub fn exercise(
    change: Change,
    constructor: impl FnOnce(
        std::sync::Arc<dyn crate::outbound::suite::interface::SuitePort>,
        std::sync::Arc<dyn LinterPort>,
        std::sync::Arc<dyn LinterPort>,
        std::sync::Arc<dyn crate::outbound::execution::interface::ExecutionPort>,
    ) -> super::use_case::CertifyUseCaseImpl,
) -> Result<crate::domain::suite::types::CaseCount, ()> {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let directory = root
        .path()
        .join(super::constants::FIXTURE_PROFILE_DIRECTORY);
    let rules = directory.join(super::constants::FIXTURE_RULE_DIRECTORY_NAME);
    std::fs::create_dir_all(rules.join(super::constants::FIXTURE_RULE_ID)).unwrap();
    let executable = root.path().join(super::constants::FIXTURE_TOOL_FILE);
    let bytes = super::constants::FIXTURE_TOOL_SCRIPT.as_bytes();
    std::fs::write(&executable, bytes).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(
        directory.join(super::constants::FIXTURE_PROFILE_FILE),
        format!(
            "tool = 'fixture'\nversion = '1'\nsha256 = '{:x}'\ndefaults = ['A']\n",
            Sha256::digest(bytes)
        ),
    )
    .unwrap();
    std::fs::write(
        rules.join(super::constants::FIXTURE_QUERY_FILE),
        super::constants::FIXTURE_QUERY_CONTENT,
    )
    .unwrap();
    std::fs::write(
        rules.join(super::constants::FIXTURE_CASES_FILE),
        toml::to_string(&case_document()).unwrap(),
    )
    .unwrap();
    let workload = root
        .path()
        .join(super::constants::FIXTURE_WORKLOAD_DIRECTORY);
    std::fs::create_dir(&workload).unwrap();
    std::fs::write(
        workload.join(super::constants::FIXTURE_PROFILE_FILE),
        super::constants::FIXTURE_PROJECT_CONTENT,
    )
    .unwrap();
    std::fs::write(
        workload.join(super::constants::FIXTURE_PATCH_FILE),
        super::constants::FIXTURE_PATCH_CONTENT,
    )
    .unwrap();
    std::fs::write(
        workload.join(super::constants::FIXTURE_COLLECTOR_FILE),
        super::constants::FIXTURE_TOOL_SCRIPT,
    )
    .unwrap();
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
        std::sync::Mutex::new(
            crate::domain::measurements::types::ExecutionProfile::Isolated(
                crate::domain::measurements::types::CollectorProfile {
                    executable: workload.join(super::constants::FIXTURE_COLLECTOR_FILE),
                    digest: crate::domain::measurements::types::CollectorDigest(format!(
                        "{:x}",
                        Sha256::digest(super::constants::FIXTURE_TOOL_SCRIPT.as_bytes())
                    )),
                },
            ),
        ),
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
        &root.path().join(super::constants::FIXTURE_CASE_DIRECTORY),
        BenchmarkCoverage::Accounted,
    )
    .unwrap();
    assert!(!certificate.fingerprint().0.is_empty());
    let measured_root = root.path().join(super::constants::FIXTURE_SOURCE_DIRECTORY);
    std::fs::create_dir(&measured_root).unwrap();
    std::fs::write(
        measured_root.join(super::constants::FIXTURE_SOURCE_FILE),
        super::constants::FIXTURE_POSITIVE_SOURCE,
    )
    .unwrap();
    let request = RunRequest {
        root: measured_root,
        files: vec![ryo::linter::RelativePath(
            super::constants::FIXTURE_SOURCE_FILE.into(),
        )],
        state: root.path().join(super::constants::FIXTURE_STATE_DIRECTORY),
    };
    let changed = match change {
        Change::None => None,
        Change::Patch => Some(workload.join(super::constants::FIXTURE_PATCH_FILE)),
        Change::Manifest => Some(workload.join(super::constants::FIXTURE_PROFILE_FILE)),
        Change::Collector => Some(workload.join(super::constants::FIXTURE_COLLECTOR_FILE)),
        Change::Instrumentation => {
            *execution.1.lock().unwrap() =
                crate::domain::measurements::types::ExecutionProfile::Wall;
            None
        }
        Change::CollectorPin | Change::CollectorPath => {
            if let crate::domain::measurements::types::ExecutionProfile::Isolated(profile) =
                &mut *execution.1.lock().unwrap()
            {
                if matches!(change, Change::CollectorPin) {
                    profile.digest.0 = super::constants::FIXTURE_ALTERED_BYTES.into();
                } else {
                    profile.executable = super::constants::FIXTURE_ALTERED_BYTES.into();
                }
            }
            None
        }
        Change::Query => Some(rules.join(super::constants::FIXTURE_QUERY_FILE)),
        Change::Executable => Some(executable),
        Change::Command => {
            std::sync::Arc::make_mut(&mut candidate)
                .arguments
                .push(super::constants::FIXTURE_ALTERED_ARGUMENT.into());
            None
        }
    };
    if let Some(path) = changed {
        std::fs::write(path, super::constants::FIXTURE_ALTERED_BYTES).unwrap();
    }
    let prior = execution.0.history.lock().unwrap().len();
    let operation = crate::inbound::use_cases::measure::use_case::MeasureUseCaseImpl::new(
        reference.clone(),
        candidate.clone(),
        execution.clone(),
        suite.clone(),
    );
    let result = crate::inbound::use_cases::measure::interface::MeasureUseCase::measure(
        &operation,
        &certificate,
        &request,
        RunOrder::ReferenceFirst,
    );
    match result {
        Ok(_) => Ok(certificate.cases()),
        Err(error) => {
            assert_eq!(
                prior,
                execution.0.history.lock().unwrap().len(),
                "{}",
                super::constants::FIXTURE_NO_EARLY_EXECUTION
            );
            let expected = match change {
                Change::Command => super::constants::FIXTURE_EXPECTED_COMMAND_REFUSAL,
                Change::Instrumentation | Change::CollectorPin | Change::CollectorPath => {
                    super::constants::FIXTURE_EXPECTED_INSTRUMENTATION_REFUSAL
                }
                _ => super::constants::FIXTURE_EXPECTED_ARTIFACT_REFUSAL,
            };
            assert_eq!(error.to_string(), expected);
            Err(())
        }
    }
}

#[cfg(not(unix))]
/// Signal that this executable fixture requires Unix process semantics.
pub fn exercise(
    _: Change,
    _constructor: impl FnOnce(
        std::sync::Arc<dyn crate::outbound::suite::interface::SuitePort>,
        std::sync::Arc<dyn LinterPort>,
        std::sync::Arc<dyn LinterPort>,
        std::sync::Arc<dyn crate::outbound::execution::interface::ExecutionPort>,
    ) -> super::use_case::CertifyUseCaseImpl,
) -> Result<crate::domain::suite::types::CaseCount, ()> {
    Err(())
}

fn case_document() -> toml::Value {
    use super::constants;
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
