//! Immutable plan layouts and shared execution accounting vocabulary.

/// How many rows a step was fed: the rows of each of its inputs, by the input's place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowsFed(pub [u64; 3]);

/// How many rows each step of a rule was fed, by the step's place.
pub struct LogicalCounts {
    pub fed: Box<[RowsFed]>,
}

/// A step of a plan, by its place among the plan's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepPlace(pub usize);

/// A plan, by where it is: the same plan has the same identity in every file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlanIdentity(pub usize);


/// Whether the rows each step is fed are counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Counting {
    /// The rows each step is fed are counted.
    Counts,
    /// They are not.
    #[default]
    DoesNotCount,
}
