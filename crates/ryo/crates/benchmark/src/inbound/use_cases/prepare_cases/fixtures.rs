//! Independent source inputs through the actual public use case and real process boundary.
#![expect(
    clippy::expect_used,
    reason = "fixture setup and fake state are test preconditions; a panic fails the unit or acceptance proof that calls this module"
)]
use crate::inbound::use_cases::certify::constants;
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, RunReport};
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use sha2::{Digest as _, Sha256};
use std::path::PathBuf;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    Complete,
    Missing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    MissingSource,
    Unexpected,
}
/// Run one public operation over independently authored source facts.
pub fn exercise(
    scenario: Scenario,
    operate: fn(&dyn SuitePort, &Path, &Path) -> Result<usize, ReportsError>,
) -> Result<usize, Refusal> {
    with_verification_sources(
        |suite, _reference, _candidate, _execution, rules, scratch, launches| {
            let input = match scenario {
                Scenario::Complete => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let result = operate(suite, &input, scratch);
            match scenario {
                Scenario::Complete => {
                    let actual = std::fs::read_to_string(launches);
                    assert_eq!(actual.unwrap_err().kind(), std::io::ErrorKind::NotFound);
                }
                Scenario::Missing => assert_eq!(
                    std::fs::symlink_metadata(launches).unwrap_err().kind(),
                    std::io::ErrorKind::NotFound
                ),
            }
            result.map_err(|error| match error {
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Refusal::MissingSource
                }
                _ => Refusal::Unexpected,
            })
        },
    )
}

#[derive(Clone, Debug)]
/// Deterministic contestant whose findings depend only on the fixture source.
pub struct Contestant {
    /// Executable identity used by preparation and certificate checks.
    pub executable: PathBuf,
    /// Mutable arguments used to prove command-profile drift is refused.
    pub arguments: Vec<std::ffi::OsString>,
}
impl LinterPort for Contestant {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
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
                    start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0),
                    end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(1),
                })
            })
            .collect();
        Ok(RunReport {
            execution_metadata:
                ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
            project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
            skipped: Vec::new(),
            coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                    selected: ryo_benchmark_contracts::domain::reports::types::PairCount(request.files.len()),
                    evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(request.files.len()),
                    skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                    unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                },
            ),
            files: request.files.clone(),
            diagnostics,
        })
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        _case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
        _profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> Result<crate::outbound::linter::types::CanonicalText, Error> {
        Ok(crate::outbound::linter::types::CanonicalText(format!("{normalized:?}")))
    }
    fn rule_context_text(
        &self,
        prepared: &ryo_benchmark_contracts::domain::context::types::PreparedRuleContext,
    ) -> crate::outbound::linter::types::CanonicalText {
        crate::outbound::linter::types::CanonicalText(format!("{prepared:?}"))
    }
}
/// Assemble the existing independent source cases and real process port for a public suite operation.
/// This helper supplies inputs and ports only; the caller invokes the actual use case.
pub fn with_verification_sources<R>(
    operation: impl FnOnce(
        &dyn crate::outbound::suite::interface::SuitePort,
        &dyn LinterPort,
        &dyn LinterPort,
        &dyn crate::outbound::execution::interface::ExecutionPort,
        &std::path::Path,
        &std::path::Path,
        &std::path::Path,
    ) -> R,
) -> R {
    let root = tempfile::tempdir().expect(constants::FIXTURE_PRECONDITION);
    let profile = root.path().join(constants::FIXTURE_PROFILE_DIRECTORY);
    let rules = profile.join(constants::FIXTURE_RULE_DIRECTORY_NAME);
    std::fs::create_dir_all(rules.join(constants::FIXTURE_RULE_ID))
        .expect(constants::FIXTURE_PRECONDITION);
    let executable = root.path().join(constants::FIXTURE_TOOL_FILE);
    let launches = root.path().join("launches");
    #[cfg(windows)]
    let script = format!(
        "@echo launch>>\"{}\"\r\n@echo fixture 1\r\n",
        launches.display()
    );
    #[cfg(not(windows))]
    let script = format!(
        "#!/bin/sh\nprintf 'launch\\n' >> '{}'\nprintf 'fixture 1\\n'\n",
        launches.display()
    );
    std::fs::write(&executable, &script).expect(constants::FIXTURE_PRECONDITION);
    executable_permissions(&executable);
    std::fs::write(
        profile.join(constants::FIXTURE_PROFILE_FILE),
        format!(
            "tool='fixture'\nversion='1'\nsha256=['{:x}']\ndefaults=['A']\n",
            Sha256::digest(script.as_bytes())
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
    let suite = crate::outbound::suite::factory::local();
    let reference = Contestant {
        executable: executable.clone(),
        arguments: Vec::new(),
    };
    let candidate = reference.clone();
    let execution = crate::outbound::execution::factory::process();
    operation(
        &suite,
        &reference,
        &candidate,
        &execution,
        &rules,
        &root.path().join("prepared"),
        &launches,
    )
}

/// Encode independent positive and negative case records.
pub fn case_document() -> toml::Value {
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
pub fn executable_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect(constants::FIXTURE_PRECONDITION);
}
/// Windows launches the fixed `.cmd` fixture without Unix executable mode bits.
#[cfg(not(unix))]
pub fn executable_permissions(_path: &std::path::Path) {}

/// The frozen workload snapshot keeps the exact patch bytes even when its source or itself is edited.
pub fn workload_snapshot_preserves_exact_patch() {
    use crate::outbound::suite::{factory::local, interface::SuitePort as _, types::{ArtifactPath, ArtifactSet}};
    let seal = |path: &std::path::Path| {
        local()
            .fingerprint(ArtifactSet(std::slice::from_ref(&path.to_path_buf())))
            .expect("seal")
    };
    let directory_guard = tempfile::tempdir().unwrap();
    let destination_guard = tempfile::tempdir().unwrap();
    let (directory, destination) = (directory_guard.path(), destination_guard.path());
    let shared = directory.join("shared");
    std::fs::create_dir_all(shared.join("patches")).expect("patch directory");
    std::fs::write(
        shared.join("benchmark.toml"),
        r#"language = "fixture"
[[projects]]
name = "project"
repository = "https://example.invalid/project.git"
revision = "revision"
patch = "patches/project.patch"
"#,
    )
    .expect("shared manifest");
    std::fs::write(shared.join("patches/project.patch"), "original patch").expect("patch");
    let frozen = destination.join("workload");
    local()
        .stage_projects(ArtifactPath(&shared), ArtifactPath(&frozen))
        .expect("freeze workload");
    let sealed = seal(&frozen);
    std::fs::write(shared.join("patches/project.patch"), "concurrent edit").expect("source edit");
    assert_eq!(
        std::fs::read_to_string(frozen.join("patches/project.patch")).unwrap(),
        "original patch"
    );
    assert_eq!(
        sealed,
        seal(&frozen)
    );
    std::fs::write(frozen.join("patches/project.patch"), "tampered snapshot")
        .expect("snapshot edit");
    assert_ne!(sealed, seal(&frozen));
}

