//! The shape every item of the `refinements` collection implements.

/// What every item of the `refinements` collection offers.
pub(crate) trait RefinementsItem {
    /// What the refinement is called when a row carries it: text that leaves the engine.
    const TEXT: &'static str;
}
