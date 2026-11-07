//! The digest port, implemented with blake3.

use crate::domain::digests::types::Digest;
use crate::domain::findings::types::{Fingerprint, Identifying};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestParts;

/// The blake3 digest adapter.
#[derive(Debug)]
pub(crate) struct DigestBlake3Adapter;

impl DigestPort for DigestBlake3Adapter {
    fn digest(&self, parts: DigestParts<'_>) -> Digest {
        let mut hasher = blake3::Hasher::new();
        let fed = parts
            .0
            .iter()
            .fold(&mut hasher, |hasher, part| hasher.update(part).update(&[0]));
        let mut text = String::with_capacity(32);
        text.extend(
            fed.finalize()
                .as_bytes()
                .iter()
                .take(16)
                .map(|byte| format!("{byte:02x}")),
        );
        Digest(text.into())
    }

    fn fingerprint(&self, identifying: &Identifying) -> Fingerprint {
        let mut hasher = blake3::Hasher::new();
        let fed = identifying
            .parts
            .0
            .iter()
            .fold(&mut hasher, |hasher, part| {
                hasher.update(part.as_bytes()).update(&[0])
            });
        let fed = if identifying.ordinal.0 > 0 {
            fed.update(&identifying.ordinal.0.to_le_bytes())
        } else {
            fed
        };
        let mut bytes = [0; 8];
        bytes.copy_from_slice(&fed.finalize().as_bytes()[..8]);
        Fingerprint(u64::from_le_bytes(bytes))
    }
}
