//! Proof-bearing measurement values issued by checked benchmark workflows.
pub mod certification_summary;
pub mod corpus_shape;
pub mod measurement_environment;
pub mod pair;
pub mod rule_tally;
pub mod sample;

/// The language as observations name it.
#[derive(Clone, Copy, Debug)]
pub struct LanguageTitle<'a>(pub &'a str);

/// Publication authority issued only after complete-inventory measurement gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementEligibility {
    /// Complete declared inventory was checked before both timed contestants.
    CompleteInventory,
    /// Explicit scoped diagnostic; never eligible for benchmark publication.
    Diagnostic,
}
