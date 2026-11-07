//! Borrowed validated ancestry: the value and its behaviour, owned together.
use crate::domain::vertices::constants::FOREST_ROOT;
use crate::domain::vertices::types::VertexId;

/// Borrowed validated ancestry. Absence is independent of the private storage sentinel.
#[derive(Clone, Copy, Debug)]
pub struct ForestParents<'a> {
    pub(crate) parents: &'a [VertexId],
}

impl ForestParents<'_> {
    /// The parent of a vertex, absent for a root.
    #[must_use]
    pub fn parent(&self, vertex: VertexId) -> Option<VertexId> {
        let parent = self.parents[vertex.0 as usize];
        (parent != FOREST_ROOT).then_some(parent)
    }
}
