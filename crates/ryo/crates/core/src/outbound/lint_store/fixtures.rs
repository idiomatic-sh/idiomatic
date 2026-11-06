//! The test support of the `lint_store` port: its contract and its fake.

pub mod contract;
pub mod fake;

/// Explicit provenance for store codec fixtures. Production captures always
/// issue proofs from the prepared rule and its actual physical coverage.
pub fn proof() -> ryo_values::domain::records::types::FrontierProof {
    ryo_values::domain::records::types::FrontierProof {
        rule: ryo_values::domain::records::types::ProducerRule(0),
        input: ryo_values::domain::records::types::FrontierOrdinal(0),
        rule_digest: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
    }
}
