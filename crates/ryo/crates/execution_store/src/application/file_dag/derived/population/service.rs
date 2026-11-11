//! The identity of an admitted population, assigned only by the engine.

/// Assigned only after application admission; canonical DAG producers keep one ID across rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PopulationId(pub(crate) usize);
