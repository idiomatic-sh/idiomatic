//! A request over one case file, a linter that reports no file, and the launches a fake execution recorded.
use crate::outbound::execution::fixtures::fake::ExecutionFake;
use benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext, RuleCoverage, RunReport,
};
use benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};

/// A request over the one case file, with no declared observations.
pub(super) fn request() -> RunRequest {
    RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: ".".into(),
        state: "cache".into(),
        files: vec![ryo::linter::RelativePath("input.py".into())],
    }
}

/// An execution that exits cleanly.
pub(super) fn succeeding() -> ExecutionFake {
    ExecutionFake {
        status: 0,
        ..Default::default()
    }
}

/// The number of launches an execution fake recorded.
pub(super) fn history_len(execution: &ExecutionFake) -> Option<usize> {
    execution.history.lock().ok().map(|history| history.len())
}

/// A contestant whose decoded report covers no requested file, over an execution that does nothing.
pub(super) struct Empty;
impl crate::outbound::linter::interface::LinterPort for Empty {
    fn rule_configuration(
        &self,
    ) -> benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
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
            execution_metadata: ExecutionMetadata::Complete,
            project_context: ProjectContext::NotRequested,
            skipped: Vec::new(),
            coverage: CoverageAccounting::Accounted(RuleCoverage {
                selected: PairCount(1),
                evaluated: PairCount(1),
                skipped: PairCount(0),
                unknown_or_unread: PairCount(0),
            }),
            diagnostics: vec![],
            files: vec![],
        })
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        _case: &benchmark_contracts::domain::workloads::types::PreparedCase,
        _profile: &benchmark_contracts::domain::workloads::types::Tool,
    ) -> Result<crate::outbound::linter::types::CanonicalText, Error> {
        Ok(crate::outbound::linter::types::CanonicalText(format!(
            "{normalized:?}"
        )))
    }
    fn rule_context_text(
        &self,
        prepared: &benchmark_contracts::domain::context::types::PreparedRuleContext,
    ) -> crate::outbound::linter::types::CanonicalText {
        crate::outbound::linter::types::CanonicalText(format!("{prepared:?}"))
    }
    fn version_line(
        &self,
        tool: &benchmark_contracts::domain::workloads::types::Tool,
    ) -> benchmark_contracts::domain::workloads::types::version_line::value::VersionLine {
        benchmark_contracts::domain::workloads::types::version_line::value::VersionLine::from(tool)
    }
}
impl crate::outbound::execution::interface::ExecutionPort for Empty {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn run(&self, _: Invocation) -> Result<ProcessOutput, Error> {
        Ok(ProcessOutput {
            signal: TerminationSignal::Exited,
            status: ProcessExitCode(0),
            stdout: CapturedBytes(vec![]),
            stderr: CapturedBytes(vec![]),
            wall: std::time::Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}

/// The environment variable names the last launch an execution fake recorded carried, sorted and joined.
pub(super) fn last_environment(execution: &ExecutionFake) -> Option<String> {
    execution.history.lock().ok().and_then(|history| {
        history.last().map(|invocation| {
            invocation
                .environment
                .keys()
                .map(|name| name.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(",")
        })
    })
}
