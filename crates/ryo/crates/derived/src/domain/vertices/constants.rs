//! The constants of the `vertices` concept.
use super::types::VertexId;

/// The parent stored for a vertex that has none; ancestry never exposes it.
pub(crate) const FOREST_ROOT: VertexId = VertexId(u32::MAX);
