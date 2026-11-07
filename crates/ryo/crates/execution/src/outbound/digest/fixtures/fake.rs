//! The fake of the `digest` port: what the core's tests stand in for it with.

use std::hash::Hasher as _;
use std::sync::atomic::{AtomicUsize, Ordering};

use rustc_hash::FxHasher;

use crate::inbound::use_cases::execution::types::digest::DigestPort;
use crate::inbound::use_cases::execution::types::digest::{DigestBytes, DigestWords};
use ryo_values::domain::stored_facts::types::Digest;

/// What the core's tests stand in for `digest` with: a fast hash, not a strong one.
#[derive(Clone, Copy, Debug)]
pub struct DigestFake;

/// The sixteen bytes two hashes of the bytes make.
fn of_bytes(bytes: &[u8]) -> Digest {
    let mut first = FxHasher::default();
    first.write(bytes);
    let mut second = FxHasher::default();
    second.write_u8(1);
    second.write(bytes);
    let mut out = [0_u8; 16];
    out[..8].copy_from_slice(&first.finish().to_le_bytes());
    out[8..].copy_from_slice(&second.finish().to_le_bytes());
    Digest(out)
}

impl DigestPort for DigestFake {
    fn digest(&self, bytes: DigestBytes<'_>) -> Digest {
        of_bytes(bytes.0)
    }

    fn digest_words(&self, words: DigestWords<'_>) -> Digest {
        let bytes: Vec<u8> = words.0.iter().flat_map(|word| word.to_le_bytes()).collect();
        of_bytes(&bytes)
    }

    fn combine(&self, digests: &[Digest]) -> Digest {
        let bytes: Vec<u8> = digests.iter().flat_map(|digest| digest.0).collect();
        of_bytes(&bytes)
    }
}

/// The fake, counting the bytes and the words it was asked to digest: what a test reads to say a run digested nothing.
#[derive(Debug, Default)]
pub struct DigestCounting {
    /// How many bytes and words it was asked to digest.
    pub digests: AtomicUsize,
}

impl DigestPort for DigestCounting {
    fn digest(&self, bytes: DigestBytes<'_>) -> Digest {
        let _before = self.digests.fetch_add(1, Ordering::Relaxed);
        DigestFake.digest(bytes)
    }

    fn digest_words(&self, words: DigestWords<'_>) -> Digest {
        let _before = self.digests.fetch_add(1, Ordering::Relaxed);
        DigestFake.digest_words(words)
    }

    fn combine(&self, digests: &[Digest]) -> Digest {
        DigestFake.combine(digests)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
