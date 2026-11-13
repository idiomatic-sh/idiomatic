//! Thin orchestration over the explicit evidence destination.
use super::interface::PublishUseCase;
use crate::domain::measurements::types::sample::value::BenchmarkSample;
use crate::outbound::evidence::interface::EvidencePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::sync::Arc;
/// The composition chooses the output destination and its lifetime.
pub struct PublishUseCaseImpl {
    evidence: Arc<dyn EvidencePort>,
}
impl PublishUseCaseImpl {
    /// Bind the required destination; no fallback destination is created.
    #[must_use]
    pub fn new(evidence: Arc<dyn EvidencePort>) -> Self {
        Self { evidence }
    }
}
impl PublishUseCase for PublishUseCaseImpl {
    fn publish(&self, samples: &[BenchmarkSample]) -> Result<(), ReportsError> {
        crate::application::publication::service::publish(self.evidence.as_ref(), samples)
    }
}

impl std::fmt::Debug for PublishUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
