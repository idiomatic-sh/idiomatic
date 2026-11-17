//! Physical fusion descriptors with their own behavior and proof.

pub(crate) mod demand;
pub(crate) mod pulled_graph;

/// Where a join decides the comparisons of its right input: in the probe, which walks the tree from each left row's node, or
/// in a filter of the right input made before the join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightPredicatePlace {
    /// The probe walks the tree by a node key and decides each comparison where the walk finds the row.
    InProbe,
    /// The comparisons are a filter of the right input, made before the join.
    BelowAsSelect,
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
