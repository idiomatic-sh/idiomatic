//! Proof-bearing measurement values issued by checked benchmark workflows.
pub mod pair;
pub mod sample;

/// Publication authority issued only after complete-inventory measurement gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementEligibility {
    /// Complete declared inventory was checked before both timed contestants.
    CompleteInventory,
    /// Explicit scoped diagnostic; never eligible for benchmark publication.
    Diagnostic,
}
