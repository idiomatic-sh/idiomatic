//! The types of the postcard store adapter: where the sections of the store file are, as the adapter reads and writes them. Plain
//! data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::stored_facts::types::Bucket;

/// A place in the store file, in bytes from its start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FileOffset(pub u64);

/// How many bytes a section of the store file holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SectionLength(pub u64);

/// The bytes of a section of the store file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SectionBytes(pub Vec<u8>);

/// Where a section is in the store file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SectionSpan {
    /// Where it starts.
    pub start: FileOffset,
    /// How long it is.
    pub length: SectionLength,
}

/// Where the words of one bucket are in the store file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BucketSpan {
    /// The bucket.
    pub bucket: Bucket,
    /// Where its section is.
    pub span: SectionSpan,
}

/// The table of contents of the store file, of the build that reads it: where each section is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TableOfContents {
    /// The small tables.
    pub tables: SectionSpan,
    /// What the files handed but their words.
    pub rest: SectionSpan,
    /// The words of each bucket, in the order of the buckets.
    pub buckets: Vec<BucketSpan>,
}

/// The store file, open, and the bytes of its table of contents.
#[derive(Debug)]
pub(super) struct OpenedStore {
    /// The file.
    pub file: std::fs::File,
    /// The bytes of its table of contents.
    pub contents: SectionBytes,
}
