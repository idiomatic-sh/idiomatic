//! Physical fusion descriptors with their own behavior and proof.

pub(crate) mod demand;
pub(crate) mod pulled_graph;

/// Whether the resident tree lookup can serve a join.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeLookup {
    /// The keys are ones the lookup implements through the view.
    Supported,
    /// The join keeps its filter as a separate select.
    Unsupported,
}

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
