//! The fake of the `digest` port: readable stand-ins, never a hash.

use crate::domain::reports::types::Digest;
use crate::domain::violations::types::{Fingerprint, SourceText};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::{DigestParts, LineFingerprints};

/// What the core's tests stand in for `digest` with: a line's fingerprint is the sum of its non-whitespace bytes,
/// and a digest the parts joined.
#[derive(Clone, Copy, Debug, Default)]
pub struct DigestFake;

impl DigestPort for DigestFake {
    fn lines(&self, text: &SourceText) -> LineFingerprints {
        LineFingerprints(
            text.0
                .split('\n')
                .map(|line| {
                    Fingerprint(
                        line.bytes()
                            .filter(|byte| !byte.is_ascii_whitespace())
                            .enumerate()
                            .map(|(index, byte)| {
                                (u64::try_from(index).unwrap_or(0) + 1) * u64::from(byte)
                            })
                            .sum(),
                    )
                })
                .collect(),
        )
    }

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
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
