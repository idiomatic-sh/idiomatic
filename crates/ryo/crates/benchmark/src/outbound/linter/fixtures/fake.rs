//! Deterministic linter output for orchestration tests.
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::linter::types::CanonicalText;
use benchmark_contracts::domain::context::types::PreparedRuleContext;
use benchmark_contracts::domain::measurements::types::Invocation;
use benchmark_contracts::domain::measurements::types::ProcessOutput;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::BenchmarkFinding;
use benchmark_contracts::domain::reports::types::RunReport;
use benchmark_contracts::domain::requests::types::RunRequest;
use benchmark_contracts::domain::workloads::types::version_line::value::VersionLine;
use benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};
use std::path::PathBuf;
#[derive(Debug, Default)]
/// Deterministic linter substitute for orchestration and contract tests.
pub struct LinterFake {
    /// Canonical findings returned by successful fixture decoding.
    pub diagnostics: Vec<BenchmarkFinding>,
    /// Labels carried through execution for call-order assertions.
    pub arguments: Vec<std::ffi::OsString>,
    /// What a run leaves in its request's state directory.
    pub store: KeptState,
}

/// What a fake run leaves in its request's state directory.
#[derive(Debug, Default)]
pub enum KeptState {
    /// Nothing: the state directory is never made.
    #[default]
    Nothing,
    /// One store file holding these bytes.
    Store(Vec<u8>),
}
impl LinterPort for LinterFake {
    fn rule_configuration(
        &self,
    ) -> benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: PathBuf::from("fixture"),
            arguments: self.arguments.clone(),
            environment: std::collections::BTreeMap::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        if output.signal
            != benchmark_contracts::domain::measurements::types::TerminationSignal::Exited
            || !(0_i32..=1_i32).contains(&output.status.0)
        {
            return Err(Error::Invalid("operational failure".into()));
        }
        if let KeptState::Store(bytes) = &self.store {
            std::fs::create_dir_all(&request.state)?;
            std::fs::write(request.state.join("store"), bytes)?;
        }
        Ok(RunReport {
            execution_metadata:
                benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
            project_context:
                benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
            files: request.files.clone(),
            skipped: Vec::new(),
            coverage: benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                benchmark_contracts::domain::reports::types::RuleCoverage {
                    selected: benchmark_contracts::domain::reports::types::PairCount(
                        request.files.len(),
                    ),
                    evaluated: benchmark_contracts::domain::reports::types::PairCount(
                        request.files.len(),
                    ),
                    skipped: benchmark_contracts::domain::reports::types::PairCount(0),
                    unknown_or_unread: benchmark_contracts::domain::reports::types::PairCount(0),
                },
            ),
            diagnostics: self.diagnostics.clone(),
        })
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        _case: &PreparedCase,
        _profile: &Tool,
    ) -> Result<CanonicalText, Error> {
        let lossy = |text: &std::ffi::OsStr| text.to_string_lossy().into_owned();
        Ok(CanonicalText(
            serde_json::json!({
                "arguments": normalized.arguments.iter().map(|argument| lossy(argument)).collect::<Vec<_>>(),
                "directory": lossy(normalized.directory.as_os_str()),
                "environment": normalized
                    .environment
                    .iter()
                    .map(|(name, value)| (lossy(name), lossy(value)))
                    .collect::<std::collections::BTreeMap<_, _>>(),
            })
            .to_string(),
        ))
    }
    fn rule_context_text(&self, prepared: &PreparedRuleContext) -> CanonicalText {
        CanonicalText(format!("{prepared:?}"))
    }
    fn version_line(&self, tool: &Tool) -> VersionLine {
        VersionLine::from(tool)
    }
}
/// The fake, decoding every report as one that observed its sealed context; the rest is [`LinterFake`].
#[derive(Debug, Default)]
pub struct ObservingLinter(pub LinterFake);
impl LinterPort for ObservingLinter {
    fn rule_configuration(
        &self,
    ) -> benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        self.0.rule_configuration()
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        self.0.prepare(request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        let mut report = self.0.decode(request, output)?;
        report.project_context =
            benchmark_contracts::domain::reports::types::ProjectContext::Observed(
                benchmark_contracts::domain::context::types::ProjectContextReceipt {
                    mode: benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk,
                    logical_root: ".".into(),
                    actual_root: request.root.clone(),
                    manifest_hash:
                        benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(
                            "fixture".into(),
                        ),
                    dependencies: std::collections::BTreeMap::new(),
                },
            );
        Ok(report)
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &PreparedCase,
        profile: &Tool,
    ) -> Result<CanonicalText, Error> {
        self.0.settings_text(normalized, case, profile)
    }
    fn rule_context_text(&self, prepared: &PreparedRuleContext) -> CanonicalText {
        self.0.rule_context_text(prepared)
    }
    fn version_line(&self, tool: &Tool) -> VersionLine {
        self.0.version_line(tool)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
