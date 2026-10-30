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
