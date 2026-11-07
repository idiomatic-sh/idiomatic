//! What the `digest` port asks of the outside, in the core's own vocabulary.

use super::types::{DigestParts, LineFingerprints};
use crate::domain::reports::types::Digest;
use crate::domain::violations::types::SourceText;

/// What every `digest` adapter implements.
pub trait DigestPort: std::fmt::Debug + Send + Sync {
    /// The fingerprint of each line of `text`: the hash of the line with every whitespace character removed, so
    /// reindenting, reflowing spaces and CRLF never change it, and no other line is read.
    fn lines(&self, text: &SourceText) -> LineFingerprints;

    /// What `parts` come to.
    fn digest(&self, parts: DigestParts<'_>) -> Digest;
}
