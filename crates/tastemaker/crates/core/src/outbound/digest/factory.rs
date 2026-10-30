//! Builds the digest adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

/// The blake3 digest adapter.
#[must_use]
pub fn blake3() -> impl crate::outbound::digest::interface::DigestPort {
    super::adapters::blake3::adapter::DigestBlake3Adapter
}
