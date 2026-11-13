//! The corpus is measured through the boundaries its composition chose, never through a sibling operation.
use super::{
    interface::MeasureCorpusUseCase,
    types::{CorpusBoundaries, CorpusMeasurement},
};
use crate::outbound::{
    context::interface::ContextPort, discovery::interface::DiscoveryPort,
    evidence::interface::EvidencePort, execution::interface::ExecutionPort,
    linter::interface::LinterPort, suite::interface::SuitePort,
    workspace::interface::WorkspacePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;
/// Measures the projects of one certified corpus.
pub struct MeasureCorpusUseCaseImpl {
    /// The pinned reference linter.
    reference: Arc<dyn LinterPort>,
    /// The candidate linter.
    candidate: Arc<dyn LinterPort>,
    /// The reference tool's own selection of a worktree's files.
    discovery: Arc<dyn DiscoveryPort>,
    /// Process launch and observation.
    execution: Arc<dyn ExecutionPort>,
    /// Artifact validation and fingerprinting.
    suite: Arc<dyn SuitePort>,
    /// Sealed project context.
    context: Arc<dyn ContextPort>,
    /// Sample publication and comparison evidence.
    evidence: Arc<dyn EvidencePort>,
    /// Corpus worktrees.
    workspace: Arc<dyn WorkspacePort>,
}
impl MeasureCorpusUseCaseImpl {
    /// Bind the boundaries chosen by the composition.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "each boundary is an independently injected port of the measurement"
    )]
    pub fn new(
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        discovery: Arc<dyn DiscoveryPort>,
        execution: Arc<dyn ExecutionPort>,
        suite: Arc<dyn SuitePort>,
        context: Arc<dyn ContextPort>,
        evidence: Arc<dyn EvidencePort>,
        workspace: Arc<dyn WorkspacePort>,
    ) -> Self {
        Self {
            reference,
            candidate,
            discovery,
            execution,
            suite,
            context,
            evidence,
            workspace,
        }
    }
}
impl MeasureCorpusUseCase for MeasureCorpusUseCaseImpl {
    fn measure_corpus(
        &self,
        corpus: &CorpusMeasurement<'_>,
        observations: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        super::service::measure_corpus(
            &CorpusBoundaries {
                reference: self.reference.as_ref(),
                candidate: self.candidate.as_ref(),
                discovery: self.discovery.as_ref(),
                execution: self.execution.as_ref(),
                suite: self.suite.as_ref(),
                context: self.context.as_ref(),
                evidence: self.evidence.as_ref(),
                workspace: self.workspace.as_ref(),
            },
            corpus,
            observations,
        )
    }
}
impl std::fmt::Debug for MeasureCorpusUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
