//! The types of the `recording` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_language::domain::relations::types::Words;
use ryo_values::domain::stored_facts::types::Bucket;

/// What a file handed to the cross-file steps in one bucket, with the numbers that differ from run to run taken out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BucketWords {
    /// The bucket.
    pub bucket: Bucket,
    /// The words.
    pub words: Words,
}

/// The words a file handed to one rule, with their buckets, one after the other: what is digested to say whether they are as they were.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Flat(pub Vec<u64>);

/// What a file hands the project, as bytes: what its interface is the digest of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InterfaceBytes(pub Vec<u8>);
