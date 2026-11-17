//! Physical fusion descriptors with their own behavior and proof.

pub(crate) mod demand;
pub(crate) mod pulled_graph;

/// Which step of the plan a closure sits at.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootStep(pub usize);

/// What an empty list of columns means to the step that holds it.
#[derive(Clone, Copy)]
pub(crate) enum EmptyColumns {
    /// An empty select keeps every column.
    KeepsEvery,
    /// An empty project keeps no column.
    KeepsNone,
}
