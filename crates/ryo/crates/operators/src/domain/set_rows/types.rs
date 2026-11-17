//! Borrowed set rows contracts; no engine authority.
/// The four set and antijoin meanings; input acquisition and delivery stay with the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetKind {
    /// Emit each distinct row from either side in encounter order.
    Union,
    /// Emit each distinct left row present on the right.
    Intersection,
    /// Emit each distinct left row absent on the right.
    Difference,
    /// Emit each left row whose projected key is absent on the right, retaining duplicates.
    Antijoin,
}

/// What a set meaning lowers to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SetLowering {
    /// Each row of the left rows, then of the right, the first time only.
    Distinct,
    /// The left rows a probe of the right ones keeps.
    Probe(crate::domain::equality_join::types::ProbeParts),
}
