//! The types of the `recording` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::stored_facts::types::{Bucket, PackedWords};

/// What a file handed to the cross-file steps in one bucket, with the numbers that differ from run to run taken out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BucketWords {
    /// The bucket.
    pub bucket: Bucket,
    /// Source-bound root and physical coverage for this exact payload.
    pub proof: ryo_values::domain::records::types::FrontierProof,
    /// Valid normalized word count, used by the unchanged digest framing.
    pub word_count: WordCount,
    /// Exact-size normalized packed words, ready to move into the store.
    pub words: PackedWords,
}

/// How many words one bucket of a file's handover holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WordCount(pub usize);

/// The words a file handed to one rule, with their buckets, one after the other: what is digested to say whether they are as they were.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Flat(pub Vec<u64>);

/// What a file hands the project, as bytes: what its interface is the digest of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InterfaceBytes(pub Vec<u8>);
