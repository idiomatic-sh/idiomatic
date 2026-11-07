//! The types of the postcard store adapter: where the sections of the store file are, as the adapter reads and writes them. Plain
//! data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::records::types::HeaderOffset;
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_findings::types::StoredFileNumber;

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
    /// The immutable snapshot length observed when this file was opened.
    pub length: SectionLength,
    /// The bytes of its table of contents.
    pub contents: SectionBytes,
}

/// The largest number a postcard varint may hold: the width of the type it is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VarintMaximum(pub u64);

/// A number read from a postcard varint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Varint(pub u64);

/// The count of one adapter's readers and writers, shared by every clone of the adapter.
#[derive(Clone, Debug, Default)]
pub(super) struct Sequence(pub std::sync::Arc<std::sync::atomic::AtomicU64>);

/// The number a reader or a writer of one adapter was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WriterNumber(pub u64);

/// One borrowed or file-backed payload, never an eagerly allocated rebuilt bucket.
#[derive(Clone, Copy, Debug)]
pub(super) enum Payload<'a> {
    /// Only adapters explicitly retaining inline data use this arm.
    Inline(&'a [u8]),
    /// A fresh payload belonging to this exact run's temporary spool.
    Staged(StagedWords),
    /// A selected row in the immutable old cache snapshot.
    Old(SectionSpan),
}

/// File identity plus the original encounter order in its bucket vector.
#[derive(Clone, Copy, Debug)]
pub(super) struct Frame<'a> {
    /// Final file number to serialize.
    pub file: StoredFileNumber,
    /// Original packed bytes or an exact byte span.
    pub payload: Payload<'a>,
}

/// One validated old row, including the framing offset needed by lazy snapshot reads.
#[derive(Clone, Copy, Debug)]
pub(super) struct OldFrame {
    /// Original file number, unchanged until application remapping.
    pub file: StoredFileNumber,
    /// Header start, before its two postcard integers.
    pub header: HeaderOffset,
    /// Original packed bytes, left in the immutable open snapshot.
    pub span: SectionSpan,
}
