//! The fake of the `digest` port: the parts written out, so a test reads what was hashed.

use crate::domain::digests::types::Digest;
use crate::domain::findings::types::{Fingerprint, Identifying};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestParts;

/// What the core's tests stand in for `digest` with: the parts joined, readable, never a hash.
#[derive(Clone, Copy, Debug, Default)]
pub struct DigestFake;

impl DigestPort for DigestFake {
    fn digest(&self, parts: DigestParts<'_>) -> Digest {
        Digest(
            parts
                .0
                .iter()
                .map(|part| String::from_utf8_lossy(part).into_owned())
                .collect::<Vec<_>>()
                .join("\u{0}")
                .into(),
        )
    }

    fn fingerprint(&self, identifying: &Identifying) -> Fingerprint {
        let length = identifying
            .parts
            .0
            .iter()
            .map(|part| part.len())
            .sum::<usize>();
        Fingerprint(u64::try_from(length).unwrap_or(u64::MAX) + identifying.ordinal.0)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
