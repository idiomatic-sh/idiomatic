//! Physical fusion descriptors with their own behavior and proof.

pub(crate) mod demand;

/// Whether the resident tree lookup can serve a join.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeLookup {
    /// The keys are ones the lookup implements through the view.
    Supported,
    /// The join keeps its filter as a separate select.
    Unsupported,
}
