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
    /// Source-backed queries checked against this fully lowered physical plan.
    pub(crate) entity_queries: std::sync::Arc<crate::domain::entity_queries::types::entity_plan::value::QueryPlan>,
    /// Each node's kernels and readers, by node id.
    pub(crate) nodes: Vec<PhysicalNode>,
    /// The shared indexes, each built once per file for all its readers: a relation's lookups, or one right side's probes.
    pub(crate) indexes: Vec<KeyedIndex>,
    /// Spike (ryo-fmx3): each node's output cell kinds, set by the engine when it assembles the program; a lazily skipped
    /// node answers an empty table of these kinds.
    pub(crate) layouts: Vec<Option<Vec<ryo_relations::domain::relations::types::CellKind>>>,
    /// Spike (ryo-fmx3): by node, the cheap leaves (lookups and scans of base rows) that must hold a row for the node to hold
    /// one: a demand-driven run reads them first.
    pub(crate) guards: Vec<Box<[u32]>>,
    /// Spike (ryo-vhuy): by node, whether the typed chain path covers it as a stage (0), else why not; set with the layouts.
    pub(crate) typed: Vec<u8>,
}

impl PhysicalPlan {
    /// Spike (ryo-fmx3): the plan with each node's output cell kinds.
    #[must_use]
    pub fn with_layouts(
        mut self,
        layouts: Vec<Option<Vec<ryo_relations::domain::relations::types::CellKind>>>,
    ) -> Self {
        self.layouts = layouts;
        self.typed = crate::domain::physical_plan::service::run::pipeline::typed::service::eligibility(&self.nodes, &self.layouts);
        self
    }

    /// Spike (ryo-vhuy): whether the typed chain path covers `node` as a stage (0), else the reason it does not.
    pub(crate) fn typed_stage(&self, node: crate::domain::physical_plan::types::PlanNodeId) -> u8 {
        self.typed.get(usize::try_from(node.0).unwrap_or(usize::MAX)).copied().unwrap_or(1)
    }

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

    /// The kind of `node`'s kernels, as the node counters report it; none for a node the plan does not hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
    /// use ryo_operators::domain::physical_plan::types::PlanNodeId;
    /// assert_eq!(PhysicalPlan::default().node_kind(PlanNodeId(0)), None);
    /// ```
    #[must_use]
    pub fn node_kind(
        &self,
        node: crate::domain::physical_plan::types::PlanNodeId,
    ) -> Option<crate::domain::physical_plan::types::node_counters::value::NodeKind> {
        self.nodes
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            .map(|physical| physical.lowering.kind())
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
