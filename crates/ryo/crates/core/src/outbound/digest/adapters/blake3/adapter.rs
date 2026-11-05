//! The digest port, implemented with blake3.

use ryo_execution::inbound::use_cases::execution::types::digest::DigestPort;
use ryo_execution::inbound::use_cases::execution::types::digest::{DigestBytes, DigestWords};
use ryo_values::domain::stored_facts::types::Digest;

/// The blake3 digest adapter.
#[derive(Debug)]
pub(crate) struct DigestBlake3Adapter;

/// The first sixteen bytes of what the hasher has come to.
fn finish(hasher: &blake3::Hasher) -> Digest {
    let hash = hasher.finalize();
    let mut out = [0_u8; 16];
    out.copy_from_slice(&hash.as_bytes()[..16]);
    Digest(out)
}

impl DigestPort for DigestBlake3Adapter {
    fn digest(&self, bytes: DigestBytes<'_>) -> Digest {
        finish(blake3::Hasher::new().update(bytes.0))
    }

    fn digest_words(&self, words: DigestWords<'_>) -> Digest {
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 4096];
        let filled = words
            .0
            .chunks(buffer.len() / 8)
            .fold(&mut hasher, |hasher, chunk| {
                for (word, slot) in chunk.iter().zip(buffer.chunks_exact_mut(8)) {
                    slot.copy_from_slice(&word.to_le_bytes());
                }
                hasher.update(buffer.get(..chunk.len() * 8).unwrap_or_default())
            });
        finish(filled)
    }

    fn combine(&self, digests: &[Digest]) -> Digest {
        let mut hasher = blake3::Hasher::new();
        let combined = digests
            .iter()
            .fold(&mut hasher, |hasher, digest| hasher.update(&digest.0));
        finish(combined)
    }
}
