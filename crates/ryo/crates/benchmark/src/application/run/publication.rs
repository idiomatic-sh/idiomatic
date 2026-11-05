//! Publication permission stays with the workflow that issued the samples.
use crate::domain::measurements::types::{MeasurementEligibility, sample::value::BenchmarkSample};
use crate::outbound::evidence::interface::EvidencePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Validate the issued proof before projecting borrowed wire input.
pub(crate) fn publish(
    evidence: &dyn EvidencePort,
    samples: &[BenchmarkSample],
) -> Result<(), Error> {
    if samples.is_empty() {
        return Err(Error::Invalid("no measured evidence".into()));
    }
    if samples
        .iter()
        .any(|sample| sample.eligibility != MeasurementEligibility::CompleteInventory)
    {
        return Err(Error::Invalid(
            "diagnostic observations cannot publish benchmark metrics".into(),
        ));
    }
    let observations = samples
        .iter()
        .map(BenchmarkSample::publication)
        .collect::<Vec<_>>();
    evidence.publish(&observations)
}
