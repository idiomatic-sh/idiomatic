//! What the `digest` port asks of the outside, in the core's own vocabulary.

use super::types::DigestParts;
use crate::domain::digests::types::Digest;
use crate::domain::findings::types::{Fingerprint, Identifying};

/// What every `digest` adapter implements.
pub trait DigestPort: Send + Sync {
    /// What the parts come to, each closed by a zero byte: the key of a cache entry or a proof.
    fn digest(&self, parts: DigestParts<'_>) -> Digest;

    /// A finding's fingerprint: its four parts, each closed by a zero byte, then its ordinal when it is not the first
    /// with those parts, hashed, as the first Tastemaker fingerprints them.
    fn fingerprint(&self, identifying: &Identifying) -> Fingerprint;
}
