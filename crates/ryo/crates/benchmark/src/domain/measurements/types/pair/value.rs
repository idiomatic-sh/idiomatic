//! Paired observations can be issued only inside the certified measurement pipeline.
#![expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-scoped fields let checked workflows issue evidence while preventing external callers from constructing certified values"
)]
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
    /// Reference observation retained after finding and source agreement.
    pub(crate) reference: Measurement,
    /// Candidate observation retained after finding and source agreement.
    pub(crate) candidate: Measurement,
}

impl Pair {
    /// Read the reference observation without permitting replacement.
    #[must_use]
    pub fn reference(&self) -> &Measurement {
        &self.reference
    }
    /// Read the candidate observation without permitting replacement.
    #[must_use]
    pub fn candidate(&self) -> &Measurement {
        &self.candidate
    }
    /// Name the two certified observations for publication.
    #[must_use]
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
