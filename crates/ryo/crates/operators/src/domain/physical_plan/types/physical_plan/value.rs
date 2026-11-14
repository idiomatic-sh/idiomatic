//! What the planner made of a logical program: the kernels of each lowered node and the nodes it could not lower yet.

use crate::domain::physical_plan::types::{KeyedIndex, Unserved};

/// The physical plan of one program. Built only by `plan`, so the engine cannot write a kernel.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// fn unlowered(plan: &PhysicalPlan) -> usize {
///     plan.unserved().len()
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct PhysicalPlan {
    /// Each node's kernels and readers, by node id.
    pub(crate) nodes: Vec<PhysicalNode>,
    /// The shared indexes, each built once per file for all its lookups.
    pub(crate) indexes: Vec<KeyedIndex>,
    /// The nodes whose kind has no lowering yet, in node order.
    pub(crate) unserved: Vec<Unserved>,
}

impl PhysicalPlan {
    /// The nodes whose kind has no lowering yet, in node order, each with the stage that lowers it. `plan` records them and
    /// never fails for them; `assemble` turns them into per-rule refusals. Branch only: deleted in F5.
    #[must_use]
    pub fn unserved(&self) -> &[Unserved] {
        &self.unserved
    }
}

/// One node of the physical plan: its kernels and who reads its rows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PhysicalNode {
    /// Its kernels.
    pub(crate) lowering: crate::domain::physical_plan::types::lowering::value::Lowering,
    /// How many nodes read its rows.
    pub(crate) readers: crate::domain::physical_plan::types::NodeReaders,
    /// Whether a consumer that reads no node keeps it past its readers.
    pub(crate) retained: crate::domain::physical_plan::types::Retention,
}
