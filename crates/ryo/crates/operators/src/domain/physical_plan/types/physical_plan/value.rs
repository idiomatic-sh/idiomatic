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

/// One node of the physical plan: its kernels and who reads its rows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PhysicalNode {
    /// Its kernels.
    pub(crate) lowering: crate::domain::physical_plan::types::lowering::value::Lowering,
    /// How many nodes read its rows.
    pub(crate) readers: crate::domain::physical_plan::types::NodeReaders,
    /// Whether a consumer that reads no node keeps it past its readers.
    pub(crate) retained: crate::domain::physical_plan::types::Retention,
    /// Whether its rows are held or streamed into its one reader.
    pub(crate) flow: crate::domain::physical_plan::types::Flow,
}
