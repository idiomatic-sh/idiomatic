//! The types of the spill adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;
use std::sync::Arc;

use ryo_values::domain::stored_facts::types::Bucket;

/// Words kept and not yet written to the spill file, as the bytes they are on disk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct UnwrittenBytes(pub Vec<u8>);

/// The room a read of a stretch fills, borrowed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RoomToFill<'a>(pub &'a mut [u8]);

/// A place in a file, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SpillOffset(pub u64);

/// How many bytes of packed words a stretch of a spill file holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PackedLength(pub u64);

/// A stretch of a spill file: the words one call of `keep` added to a bucket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpillStretch {
    /// Where the stretch starts.
    pub offset: SpillOffset,
    /// How many bytes of packed words it holds.
    pub length: PackedLength,
}

/// The file the words of a worker are kept in.
#[derive(Debug, Default)]
pub(crate) enum SpillFile {
    /// No file yet: it is made when the words waiting first reach the bound.
    #[default]
    Missing,
    /// The file, open.
    Open(Arc<std::fs::File>),
}

/// The words a worker added to buckets when the adapter keeps them in a file and not in memory: the file, what has not yet been
/// written to it, and where each bucket's words are in it.
#[derive(Debug, Default)]
pub(crate) struct Spill {
    /// The file the words are kept in.
    pub file: SpillFile,
    /// Words kept and not yet written, as bytes.
    pub pending: UnwrittenBytes,
    /// How many bytes the file holds.
    pub written: SpillOffset,
    /// The stretches of each bucket, in the order they were kept.
    pub stretches: BTreeMap<Bucket, Vec<SpillStretch>>,
}

/// Which worker of the adapter a spill file belongs to, in the order the workers were opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SpillWorkerNumber(pub usize);
