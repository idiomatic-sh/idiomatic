//! Paired observations can be issued only inside the certified measurement pipeline.
use crate::domain::measurements::types::{Measurement, SampleName, sample::value::BenchmarkSample};

/// Reference and candidate observations retained after exact agreement.
///
/// ```compile_fail
/// use ryo_benchmark::domain::measurements::types::{Measurement, pair::value::Pair};
/// fn forge(reference: Measurement, candidate: Measurement) -> Pair {
///     Pair { reference, candidate }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Pair {
    pub(crate) reference: Measurement,
    pub(crate) candidate: Measurement,
}

impl Pair {
    /// Read the reference observation without permitting replacement.
    pub fn reference(&self) -> &Measurement {
        &self.reference
    }
    /// Read the candidate observation without permitting replacement.
    pub fn candidate(&self) -> &Measurement {
        &self.candidate
    }
    /// Name the two certified observations for publication.
    pub fn samples(self, names: [SampleName; 2]) -> [BenchmarkSample; 2] {
        let [reference, candidate] = names;
        [
            BenchmarkSample {
                name: reference,
                measurement: self.reference,
            },
            BenchmarkSample {
                name: candidate,
                measurement: self.candidate,
            },
        ]
    }
}
