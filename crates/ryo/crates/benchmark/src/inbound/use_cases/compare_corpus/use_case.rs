//! The `compare_corpus` use case: the struct holding only the ports it calls.
use super::interface::CompareCorpusUseCase;
use super::types::{CorpusParity, ParityBoundaries};
use crate::outbound::{
    context::interface::ContextPort, evidence::interface::EvidencePort,
    execution::interface::ExecutionPort, linter::interface::LinterPort,
    suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;

/// `compare_corpus`, over only the ports it calls, each injected by the edge.
pub struct CompareCorpusUseCaseImpl {
    /// The pinned reference linter.
    reference: Arc<dyn LinterPort>,
    /// The candidate linter.
    candidate: Arc<dyn LinterPort>,
    /// Process launch and observation.
    execution: Arc<dyn ExecutionPort>,
    /// Sealed project context.
    context: Arc<dyn ContextPort>,
    /// Accepted differences and source text.
    suite: Arc<dyn SuitePort>,
    /// Parity metrics and the per-rule document.
    evidence: Arc<dyn EvidencePort>,
}

impl CompareCorpusUseCaseImpl {
    /// `compare_corpus` over the ports it calls, which the edge injects.
    #[must_use]
    pub fn new(
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        execution: Arc<dyn ExecutionPort>,
        context: Arc<dyn ContextPort>,
        suite: Arc<dyn SuitePort>,
        evidence: Arc<dyn EvidencePort>,
    ) -> Self {
        Self {
            reference,
            candidate,
            execution,
            context,
            suite,
            evidence,
        }
    }
}

impl CompareCorpusUseCase for CompareCorpusUseCaseImpl {
    fn compare_corpus(
        &self,
        parity: &CorpusParity<'_>,
        observations: &mut dyn std::io::Write,
    ) -> Result<(), Error> {
        super::service::compare_corpus(
            &ParityBoundaries {
                reference: self.reference.as_ref(),
                candidate: self.candidate.as_ref(),
                execution: self.execution.as_ref(),
                context: self.context.as_ref(),
                suite: self.suite.as_ref(),
                evidence: self.evidence.as_ref(),
            },
            parity,
            observations,
        )
    }
}

impl std::fmt::Debug for CompareCorpusUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
