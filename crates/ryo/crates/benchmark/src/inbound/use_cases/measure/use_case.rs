//! Both contestants use the same execution implementation.
use super::interface::MeasureUseCase;
use crate::domain::measurements::types::pair::value::Pair;
use crate::domain::suite::types::certificate::value::Certificate;
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::{
    context::interface::ContextPort, evidence::interface::EvidencePort,
    execution::interface::ExecutionPort, linter::interface::LinterPort,
};
use ryo_benchmark_contracts::domain::measurements::types::RunOrder;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::sync::Arc;
/// Release paired process observations only after the correctness gate succeeds.
pub struct MeasureUseCaseImpl {
    /// Pinned reference tool preparation and decoding boundary.
    reference: Arc<dyn LinterPort>,
    /// Required workload meaning, independent of timing/collector choices.
    mode: ryo_benchmark_contracts::domain::measurements::types::MeasurementMode,
    /// Candidate tool preparation and decoding boundary.
    candidate: Arc<dyn LinterPort>,
    /// Process launch and resource observation boundary.
    execution: Arc<dyn ExecutionPort>,
    /// Artifact validation and exact content fingerprinting boundary.
    suite: Arc<dyn SuitePort>,
    /// Materialized-context validation boundary.
    context: Arc<dyn ContextPort>,
    /// Comparison evidence boundary written beside timing output.
    evidence: Arc<dyn EvidencePort>,
}
impl MeasureUseCaseImpl {
    /// Construct this operation with the dependencies chosen by its composition.
    #[must_use]
    pub fn new(
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        execution: Arc<dyn ExecutionPort>,
        suite: Arc<dyn SuitePort>,
        context: Arc<dyn ContextPort>,
        evidence: Arc<dyn EvidencePort>,
        mode: ryo_benchmark_contracts::domain::measurements::types::MeasurementMode,
    ) -> Self {
        Self {
            reference,
            candidate,
            execution,
            suite,
            context,
            evidence,
            mode,
        }
    }
}
impl MeasureUseCase for MeasureUseCaseImpl {
    fn measure(
        &self,
        certificate: &Certificate,
        request: &RunRequest,
        order: RunOrder,
    ) -> Result<Pair, Error> {
        super::service::measure_certified(
            certificate,
            &self.mode,
            self.suite.as_ref(),
            self.context.as_ref(),
            self.evidence.as_ref(),
            self.reference.as_ref(),
            self.candidate.as_ref(),
            self.execution.as_ref(),
            request,
            order,
        )
    }
}

impl std::fmt::Debug for MeasureUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
