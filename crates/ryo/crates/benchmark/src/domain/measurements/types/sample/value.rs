//! A publication sample cannot be manufactured from an uncertified observation.
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
    pub(crate) name: SampleName,
    pub(crate) measurement: Measurement,
}

impl BenchmarkSample {
    /// Stable identity supplied when the certified pair was named.
    pub fn name(&self) -> &SampleName {
        &self.name
    }
    /// The original, immutable certified observation.
    pub fn measurement(&self) -> &Measurement {
        &self.measurement
    }
}
