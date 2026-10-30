//! The types of the `allocation_verdict` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

/// How many blocks were allocated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AllocatedBlocks(pub u64);

/// The blocks each job allocated, in the order the jobs ran.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct JobBlocks(pub Vec<u64>);

/// The bytes of an input, which names the file the parser library read.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InputBytes(pub Vec<u8>);

/// The blocks the parser library allocates for each input, on its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParserBlocks(pub BTreeMap<InputBytes, AllocatedBlocks>);

/// The blocks each judged file allocated beyond the parser library.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Differences(pub Vec<AllocatedBlocks>);

/// How many files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileCount(pub usize);

/// What the differences come to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AllocationVerdict {
    /// No file allocated more than allowed, and no more files allocated at all than the findings explain.
    Within,
    /// A file allocated more than one block beyond the parser library.
    BeyondOneBlock,
    /// More files allocated beyond the parser library than a vector that doubles explains.
    TooManyGrew {
        /// How many allocated beyond it.
        grew: FileCount,
        /// How many were judged.
        judged: FileCount,
    },
}
