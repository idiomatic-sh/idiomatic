//! Paired observations can be issued only inside the certified measurement pipeline.
#![expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-scoped fields let checked workflows issue evidence while preventing external callers from constructing certified values"
)]
use crate::domain::measurements::types::sample::value::BenchmarkSample;
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::SampleName;

/// Reference and candidate observations retained under the explicit diagnostics comparison contract.
///
/// ```compile_fail,E0451
/// use ryo_benchmark_contracts::domain::measurements::types::Measurement;
/// use ryo_benchmark::domain::measurements::types::pair::value::Pair;
/// fn forge(reference: Measurement, candidate: Measurement, from: Pair) -> Pair {
///     Pair { reference, candidate, ..from }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Pair {
    /// Unforgeable publication authority retained from the checked request.
    pub(crate) eligibility: crate::domain::measurements::types::MeasurementEligibility,
    /// Reference observation retained after finding and source agreement.
    pub(crate) reference: Measurement,
    /// Candidate observation retained after finding and source agreement.
    pub(crate) candidate: Measurement,
}

impl Pair {
    /// Read the independent evidence dimensions; diagnostic certification is not a full execution-parity claim.
    #[must_use]
    pub fn comparison(&self) -> ryo_benchmark_contracts::domain::reports::types::ReportComparison {
        crate::domain::diagnostics::service::compare_dimensions(
            &self.reference.report,
            &self.candidate.report,
        )
    }
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
                eligibility: self.eligibility,
                name: reference,
                measurement: self.reference,
            },
            BenchmarkSample {
                eligibility: self.eligibility,
                name: candidate,
                measurement: self.candidate,
            },
        ]
    }
}
