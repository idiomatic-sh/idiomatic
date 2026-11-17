//! The planner's one dispatch over the operation kinds, each kind lowered once by its own concept; the registry of the rewrites
//! it applies over what it lowered, and the `kernels!` catalog of the kernels the lowerings run.
pub(crate) mod indexes;
pub(crate) mod kernels;
pub(crate) mod lowering;
pub(crate) mod numbers;
pub(crate) mod run;

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem;
use crate::domain::physical_plan::growth;
use crate::domain::physical_plan::service::lowering::service::lowered;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::lowering::value::{Lowering, ProbeNode, ProjectNode};
use crate::domain::physical_plan::types::physical_plan::value::{PhysicalNode, PhysicalPlan};
use crate::domain::physical_plan::types::{
    EngineUses, FirstRead, KeyedIndex, NodeFlow, NodeReaders, PlanInput, PlanNodeId, ReadersFound,
    RowSource,
};
use ryo_values::domain::plan_parts::types::{InputNumber, RepeatedRows};

/// Every rewrite, in the order the planner applies them: each an equivalence, so a plan answers the same rows with or without
/// it.
pub(crate) const REWRITES: &[&dyn RewritesItem] = &[
    &crate::domain::physical_plan::collections::rewrites::keyed_select::item::KeyedSelectItem,
    &crate::domain::physical_plan::collections::rewrites::ordered_existence::item::OrderedExistenceItem,
    &crate::domain::physical_plan::collections::rewrites::existence_probe::item::ExistenceProbeItem,
    &crate::domain::physical_plan::collections::rewrites::tree_probe::item::TreeProbeItem,
];

/// Declares the kernels once: the catalog the reachability case reads, each kernel by its item.
macro_rules! kernels {
    ($( $(#[$doc:meta])* $kernel:ident => $item:path, )+) => {
        /// A kernel of the physical plan.
        #[cfg(test)]
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(crate) enum Kernel {
            $( $(#[$doc])* $kernel, )+
        }

                #[cfg(test)]
        impl Kernel {
            /// Every kernel, in catalog order.
            pub(crate) const ALL: &'static [Self] = &[ $( Self::$kernel, )+ ];

            /// The item of the kernel.
            pub(crate) fn item(self) -> &'static dyn crate::domain::physical_plan::collections::kernels::interface::KernelsItem {
                match self {
                    $( Self::$kernel => &$item, )+
                }
            }
        }
    };
}

kernels! {
    /// The `scan_facts` kernel.
    ScanFacts => crate::domain::physical_plan::collections::kernels::scan_facts::item::ScanFactsItem,
    /// The `scan_roots` kernel.
    ScanRoots => crate::domain::physical_plan::collections::kernels::scan_roots::item::ScanRootsItem,
    /// The `scan_node` kernel.
    ScanNode => crate::domain::physical_plan::collections::kernels::scan_node::item::ScanNodeItem,
    /// The `scan_bound` kernel.
    ScanBound => crate::domain::physical_plan::collections::kernels::scan_bound::item::ScanBoundItem,
    /// The `lookup` kernel.
    Lookup => crate::domain::physical_plan::collections::kernels::lookup::item::LookupItem,
    /// The `filter` kernel.
    Filter => crate::domain::physical_plan::collections::kernels::filter::item::FilterItem,
    /// The `project` kernel.
    Project => crate::domain::physical_plan::collections::kernels::project::item::ProjectItem,
    /// The `extend` kernel.
    Extend => crate::domain::physical_plan::collections::kernels::extend::item::ExtendItem,
    /// The `probe` kernel.
    Probe => crate::domain::physical_plan::collections::kernels::probe::item::ProbeItem,
    /// The `probe_right` kernel.
    ProbeRight => crate::domain::physical_plan::collections::kernels::probe_right::item::ProbeRightItem,
    /// The `hash_index` kernel.
    HashIndex => crate::domain::physical_plan::collections::kernels::hash_index::item::HashIndexItem,
    /// The `distinct` kernel.
    Distinct => crate::domain::physical_plan::collections::kernels::distinct::item::DistinctItem,
    /// The `group` kernel.
    Group => crate::domain::physical_plan::collections::kernels::group::item::GroupItem,
    /// The `text_map` kernel.
    TextMap => crate::domain::physical_plan::collections::kernels::text_map::item::TextMapItem,
    /// The `band_join` kernel.
    BandJoin => crate::domain::physical_plan::collections::kernels::band_join::item::BandJoinItem,
    /// The `ordered_match` kernel.
    OrderedMatch => crate::domain::physical_plan::collections::kernels::ordered_match::item::OrderedMatchItem,
    /// The `render` kernel.
    Render => crate::domain::physical_plan::collections::kernels::render::item::RenderItem,
    /// The `reach` kernel.
    Reach => crate::domain::physical_plan::collections::kernels::reach::item::ReachItem,
    /// The `directed_walk` kernel.
    DirectedWalk => crate::domain::physical_plan::collections::kernels::directed_walk::item::DirectedWalkItem,
    /// The `ordered_walk` kernel.
    OrderedWalk => crate::domain::physical_plan::collections::kernels::ordered_walk::item::OrderedWalkItem,
    /// The `components` kernel.
    Components => crate::domain::physical_plan::collections::kernels::components::item::ComponentsItem,
    /// The `queued_walk` kernel.
    QueuedWalk => crate::domain::physical_plan::collections::kernels::queued_walk::item::QueuedWalkItem,
}

/// The physical plan of `program`: each node lowered once, and every registered rewrite applied.
///
/// # Errors
///
/// `WrongOperation` when a node reads a later node.
pub(crate) fn plan(program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault> {
    plan_with(program, REWRITES)
}

/// The physical plan of `program` with `rewrites` applied in order, and no other.
///
/// # Errors
///
/// `WrongOperation` when a node reads a later node; `NotRun` when a node does not read the inputs its kind needs, so no kernel
/// can make its rows.
pub(crate) fn plan_with(
    program: &LogicalProgram<'_>,
    rewrites: &[&dyn RewritesItem],
) -> Result<PhysicalPlan, PortableOperatorFault> {
    program.check_shape()?;
    let logical = program.nodes();
    let mut nodes: Vec<PhysicalNode> = logical
        .iter()
        .map(|node| {
            lowered(node.operation, node.inputs.0, node.bindings.0)
                .map(|lowering| PhysicalNode {
                    lowering,
                    readers: NodeReaders(0),
                    engine: EngineUses(0),
                    flow: NodeFlow::Held,
                })
                .ok_or(PortableOperatorFault::NotRun)
        })
        .collect::<Result<_, _>>()?;
    for node in logical {
        for input in node.inputs.0 {
            if let PlanInput::Node(read) = input
                && let Some(reader) = place(&mut nodes, *read)
            {
                reader.readers = NodeReaders(reader.readers.0.saturating_add(1));
            }
        }
    }
    for kept in program.emitted().chain(program.held().iter().copied()) {
        if let Some(node) = place(&mut nodes, kept) {
            node.engine = EngineUses(node.engine.0.saturating_add(1));
        }
    }
    let mut indexes = Vec::new();
    for rewrite in rewrites {
        rewrite.rewrite(&mut nodes, &mut indexes);
    }
    // After the rewrites: a node a rewrite left unread is not judged, and a probe it made is judged as it now runs.
    let _bounds = growth::service::checked(&nodes).map_err(PortableOperatorFault::Superlinear)?;
    // After the rewrites: a rewrite changes a probe's right rows, key and mode, and replaces probes.
    indexes::service::shared_indexes(&mut nodes, &mut indexes);
    // Last: whether a node streams reads every node's final sources and the indexes.
    flows(&mut nodes, &indexes);
    Ok(PhysicalPlan { nodes, indexes })
}

/// Streams each node whose rows only one streaming stage reads, first, so no one else ever asks for them (`flow_of`). Who reads
/// each node, and which rows a shared index keys, are found once for the plan, so deciding every node's flow is linear in it.
fn flows(nodes: &mut [PhysicalNode], indexes: &[KeyedIndex]) {
    let mut reads = vec![ReadersFound::default(); nodes.len()];
    for (reader, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        for (place, source) in reader.lowering.sources().enumerate() {
            if let RowSource::Node(read) = source
                && let Some(slot) = reads.get_mut(usize::try_from(read.0).unwrap_or(usize::MAX))
            {
                slot.count = NodeReaders(slot.count.0.saturating_add(1));
                if slot.first.is_none() {
                    slot.first = Some(FirstRead {
                        reader: id,
                        place: InputNumber(place),
                    });
                }
            }
        }
    }
    let indexed: hashbrown::HashSet<RowSource> = indexes.iter().map(|index| index.source).collect();
    let decided: Vec<NodeFlow> = reads
        .iter()
        .zip((0..).map(PlanNodeId))
        .map(|(read, id)| flow_of(nodes, id, read, &indexed))
        .collect();
    for (node, flow) in nodes.iter_mut().zip(decided) {
        node.flow = flow;
    }
}

/// The flow of node `id`, which `read` says who reads. It streams iff all hold:
///  (1) it is an intermediate streaming stage: a filter, an extension, a `text_map` stage, or a projection or a probe that
///      keeps repeated rows (one that drops them keys the rows it keeps, so it streams only as a chain's tail);
///  (2) one node reads it, once, and no engine use keeps it (it is neither emitted nor held);
///  (3) its one reader is a streaming stage (a projection or a probe of either kind) whose first source, a probe's left,
///      is this node: a probe's right side is never streamed, its index blocks;
///  (4) no shared index keys its rows (`indexed`).
/// Every other node is held.
fn flow_of(
    nodes: &[PhysicalNode],
    id: PlanNodeId,
    read: &ReadersFound,
    indexed: &hashbrown::HashSet<RowSource>,
) -> NodeFlow {
    let Some(node) = nodes.get(usize::try_from(id.0).unwrap_or(usize::MAX)) else {
        return NodeFlow::Held;
    };
    let read_first_by_a_stage = read.count == NodeReaders(1)
        && read.first.is_some_and(|first| {
            first.place == InputNumber(0)
                && nodes
                    .get(usize::try_from(first.reader.0).unwrap_or(usize::MAX))
                    .is_some_and(|reader| {
                        matches!(
                            reader.lowering,
                            Lowering::Filter(_)
                                | Lowering::Project(_)
                                | Lowering::Extend(_)
                                | Lowering::TextMap(_)
                                | Lowering::Probe(_)
                        )
                    })
        });
    let stage = match &node.lowering {
        Lowering::Filter(_) | Lowering::Extend(_) | Lowering::TextMap(_) => true,
        Lowering::Project(ProjectNode { project, .. }) => project.distinct == RepeatedRows::Kept,
        Lowering::Probe(ProbeNode { probe, .. }) => probe.repeated == RepeatedRows::Kept,
        Lowering::Lookup(_)
        | Lowering::Distinct(_)
        | Lowering::Group(_)
        | Lowering::Reach(_)
        | Lowering::SeededReach(_)
        | Lowering::Axis(_)
        | Lowering::OrderedWalk(_)
        | Lowering::Worklist(_)
        | Lowering::Components(_)
        | Lowering::OrderedMatch(_)
        | Lowering::BandJoin(_)
        | Lowering::Render(_)
        | Lowering::ScanBound(_) => false,
    };
    if stage
        && node.readers == NodeReaders(1)
        && node.engine == EngineUses(0)
        && read_first_by_a_stage
        && !indexed.contains(&RowSource::Node(id))
    {
        NodeFlow::Streamed
    } else {
        NodeFlow::Held
    }
}

/// The node at `id`, to change.
fn place(nodes: &mut [PhysicalNode], id: PlanNodeId) -> Option<&mut PhysicalNode> {
    nodes.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
}
