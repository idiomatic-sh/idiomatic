//! Publication permission stays with the workflow that issued the samples.
use crate::domain::measurements::types::{MeasurementEligibility, sample::value::BenchmarkSample};
use crate::outbound::evidence::interface::EvidencePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Validate the issued proof before projecting borrowed wire input.
///
/// # Errors
/// Refuses an empty sample set, samples without complete-inventory eligibility, and a failed publication.
pub fn publish(evidence: &dyn EvidencePort, samples: &[BenchmarkSample]) -> Result<(), Error> {
    if samples.is_empty() {
        return Err(Error::Invalid(ryo_benchmark_contracts::domain::reports::constants::NO_MEASURED_EVIDENCE.into()));
    }
    if samples
        .iter()
        .any(|sample| sample.eligibility != MeasurementEligibility::CompleteInventory)
    {
        return Err(Error::Invalid(
            ryo_benchmark_contracts::domain::reports::constants::DIAGNOSTIC_OBSERVATIONS_CANNOT_PUBLISH.into(),
        ));
    }
    let observations = samples
        .iter()
        .map(BenchmarkSample::publication)
        .collect::<Vec<_>>();
    evidence.publish(
        &crate::outbound::evidence::types::measured_publication::value::MeasuredPublication::checked(&observations),
    )
}
