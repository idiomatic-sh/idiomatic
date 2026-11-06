//! Physical fusion descriptors with their own behavior and proof.

use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

pub(crate) mod demand;

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

/// Whether a chain of steps ends at a stored relation the closure can be pulled to.
#[derive(Clone, Copy)]
pub(crate) enum GraphInput {
    /// A stored relation, or a pure chain over one.
    Stored,
    /// Anything else.
    Unsupported,
}

/// Whether a comparison is safe to evaluate in another place.
#[derive(Clone, Copy)]
pub(crate) enum Purity {
    /// Plain values and plain tests.
    Pure,
    /// A function or pattern that can fail.
    ErrorSensitive,
}

/// The graph a closure reads, with its endpoints and partition keys, after pulling through views.
pub(crate) struct GraphSource {
    /// The pulled-through input.
    pub input: Input,
    /// The source endpoint column.
    pub source: ColumnIndex,
    /// The destination endpoint column.
    pub destination: ColumnIndex,
    /// The partition keys.
    pub keys: Vec<ColumnIndex>,
}
