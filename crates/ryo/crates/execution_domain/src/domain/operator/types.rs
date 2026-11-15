//! Immutable plan layouts, shared execution accounting vocabulary and what an operator reads.

/// How many rows a step was fed: the rows of each of its inputs, by the input's place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowsFed(pub [u64; 3]);

/// How many rows each step of a rule was fed, by the step's place.
pub struct LogicalCounts {
    /// The rows fed to each step, by the step's place.
    pub fed: Box<[RowsFed]>,
}
