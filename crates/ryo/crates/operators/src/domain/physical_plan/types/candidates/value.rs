//! The `Candidates` value.

use ryo_relations::domain::relations::types::NodeId;

/// The tree nodes a root may start from, in tree order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidates<'a>(pub &'a [NodeId]);

impl<'a> Candidates<'a> {
    /// The tree nodes, in tree order.
    #[must_use]
    pub const fn nodes(self) -> &'a [NodeId] {
        self.0
    }
}
