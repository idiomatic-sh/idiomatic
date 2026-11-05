//! The `stored_facts` service: free functions named by verb over what a run keeps of the words and the text of a file, with no IO and no ports.

use super::types::{Digest, DigestBlankness};

/// Whether a digest is the blank one, which a record keeps for what was never digested.
#[must_use]
pub fn blankness_of(digest: &Digest) -> DigestBlankness {
    if digest.0 == [0; 16] {
        DigestBlankness::Blank
    } else {
        DigestBlankness::Filled
    }
}

/// The bucket the rows of the frontier's input at `input` of a rule wait in, the rule's first bucket being `first`.
#[must_use]
pub fn frontier_bucket(
    first: super::types::Bucket,
    input: crate::domain::plan_parts::types::InputNumber,
) -> super::types::Bucket {
    super::types::Bucket(
        first
            .0
            .saturating_add(u32::try_from(input.0).unwrap_or(u32::MAX)),
    )
}
