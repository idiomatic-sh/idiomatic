//! What the `digest` port asks of the outside, in the core's own vocabulary.

use super::types::{DigestBytes, DigestWords};
use ryo_values::domain::stored_facts::types::Digest;

/// What every `digest` adapter implements.
pub trait DigestPort: Send + Sync {
    /// What `bytes` come to.
    fn digest(&self, bytes: DigestBytes<'_>) -> Digest;

    /// What the words come to, as the bytes they are on a little-endian machine, whatever the machine.
    fn digest_words(&self, words: DigestWords<'_>) -> Digest;

    /// What the digests come to, in their order.
    fn combine(&self, digests: &[Digest]) -> Digest;
}
