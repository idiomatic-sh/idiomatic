//! Builds the digest adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The blake3 digest adapter, from what the edge passes.
#[must_use]
pub fn blake3() -> impl crate::outbound::digest::interface::DigestPort {
    super::adapters::blake3::adapter::DigestBlake3Adapter
}
