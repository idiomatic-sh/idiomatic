//! A publication sample cannot be manufactured from an uncertified observation.
#![expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-scoped fields let checked workflows issue evidence while preventing external callers from constructing certified values"
)]
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::SampleName;

/// Immutable named observation issued from a certified pair.
///
/// ```compile_fail
/// use ryo_benchmark_contracts::domain::measurements::types::{Measurement, SampleName};
/// use ryo_benchmark::domain::measurements::types::sample::value::BenchmarkSample;
/// fn forge(name: SampleName, measurement: Measurement) -> BenchmarkSample {
///     BenchmarkSample { name, measurement }
/// }
/// ```
///
/// ```compile_fail
/// use ryo_benchmark_contracts::domain::measurements::types::Measurement;
/// use ryo_benchmark::domain::measurements::types::sample::value::BenchmarkSample;
/// fn replace(sample: &mut BenchmarkSample, forged: Measurement) {
///     sample.measurement = forged;
/// }
/// ```
#[derive(Clone, Debug)]
pub struct BenchmarkSample {
    /// Eligibility carried from the issued pair, never supplied by an external caller.
    pub(crate) eligibility: crate::domain::measurements::types::MeasurementEligibility,
    /// Project, contestant and cache-condition identity assigned to the pair.
    pub(crate) name: SampleName,
    /// Original observation from the certified pair.
    pub(crate) measurement: Measurement,
}

impl BenchmarkSample {
    /// Stable identity supplied when the certified pair was named.
    #[must_use]
    pub fn name(&self) -> &SampleName {
        &self.name
    }
    /// The original, immutable certified observation.
    #[must_use]
    pub fn measurement(&self) -> &Measurement {
        &self.measurement
    }
}

impl BenchmarkSample {
    /// Borrow the observed output; this projection cannot replace a sample or issue a certificate.
    #[must_use]
    pub fn publication(
        &self,
    ) -> ryo_benchmark_contracts::domain::measurements::types::PublicationSample<'_> {
        ryo_benchmark_contracts::domain::measurements::types::PublicationSample {
            complete_inventory: self.eligibility
                == crate::domain::measurements::types::MeasurementEligibility::CompleteInventory,
            name: &self.name,
            measurement: &self.measurement,
        }
    }
}
