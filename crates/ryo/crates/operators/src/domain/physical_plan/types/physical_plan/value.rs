//! What the planner made of a logical program: the kernels of each node and the indexes they share.

use crate::domain::physical_plan::types::KeyedIndex;

/// The physical plan of one program. Built only by `plan`, so the engine cannot write a kernel.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// fn planned(plan: &PhysicalPlan) -> &PhysicalPlan {
///     plan
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct PhysicalPlan {
    /// Each node's kernels and readers, by node id.
    pub(crate) nodes: Vec<PhysicalNode>,
    /// The shared indexes, each built once per file for all its readers: a relation's lookups, or one right side's probes.
    pub(crate) indexes: Vec<KeyedIndex>,
}

impl PhysicalPlan {
    /// How many nodes the plan runs: one per node of the program it was made of.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
    /// use ryo_operators::domain::physical_plan::types::NodeTotal;
    /// assert_eq!(PhysicalPlan::default().node_count(), NodeTotal(0));
    /// ```
    #[must_use]
    pub fn node_count(&self) -> crate::domain::physical_plan::types::NodeTotal {
        crate::domain::physical_plan::types::NodeTotal(self.nodes.len())
    }
}

/// One node of the physical plan: its kernels and who reads its rows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PhysicalNode {
    /// Its kernels.
    pub(crate) lowering: crate::domain::physical_plan::types::lowering::value::Lowering,
    /// How many nodes read its rows.
    pub(crate) readers: crate::domain::physical_plan::types::NodeReaders,
    /// How many engine uses it has: rules that emit it and consumers that hold it.
    pub(crate) engine: crate::domain::physical_plan::types::EngineUses,
    /// Whether its rows are held or streamed into its one reader.
    pub(crate) flow: crate::domain::physical_plan::types::NodeFlow,
}
