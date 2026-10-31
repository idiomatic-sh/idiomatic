//! A publication sample cannot be manufactured from an uncertified observation.
#![expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-scoped fields let checked workflows issue evidence while preventing external callers from constructing certified values"
)]
use crate::domain::measurements::types::{Measurement, SampleName};

/// Immutable named observation issued from a certified pair.
///
/// ```compile_fail
/// use ryo_benchmark::domain::measurements::types::{Measurement, SampleName, sample::value::BenchmarkSample};
/// fn forge(name: SampleName, measurement: Measurement) -> BenchmarkSample {
///     BenchmarkSample { name, measurement }
/// }
/// ```
///
/// ```compile_fail
/// use ryo_benchmark::domain::measurements::types::{Measurement, sample::value::BenchmarkSample};
/// fn replace(sample: &mut BenchmarkSample, forged: Measurement) {
///     sample.measurement = forged;
/// }
/// ```
#[derive(Clone, Debug)]
pub struct BenchmarkSample {
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
