//! The types of the `planning` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A fixture's project planned: the plan of its lint calls, and what preparing it found.
pub(crate) struct ProjectPlan {
    /// The plan of the lint calls.
    pub plan: crate::domain::applications::types::Plan,
    /// What preparing it found.
    pub findings: Vec<crate::domain::findings::types::Finding>,
}
