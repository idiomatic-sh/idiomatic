//! Builds the sarif adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

/// The serde-sarif adapter: SARIF 2.1.0 read and written through `serde_sarif`'s model of it.
#[must_use]
pub fn serde_sarif() -> impl crate::outbound::sarif::interface::SarifPort {
    super::adapters::serde_sarif::adapter::SarifSerdeSarifAdapter
}
