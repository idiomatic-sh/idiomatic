//! The types of the `walks` part: a walk's two ends, and the tree a nearest walk climbs.

use crate::domain::lowering::flattening::types::{FlatTerm, NearestDomain, NearestOver};

/// A nearest walk: what is walked toward the root, and what holds its domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NearestClimb {
    /// What is walked toward the root.
    pub(crate) over: NearestOver,
    /// What holds the elements a walk may stop at.
    pub(crate) domain: NearestDomain,
}

/// The two ends of a walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WalkEnds {
    /// The element walked from.
    pub(crate) start: FlatTerm,
    /// The element reached.
    pub(crate) reached: FlatTerm,
}
