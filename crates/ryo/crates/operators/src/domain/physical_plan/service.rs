//! The planner's one dispatch over the operation kinds, each kind lowered once by its own concept; the registry of the rewrites
//! it applies over what it lowered, and the `kernels!` catalog of the kernels the lowerings run.
pub(crate) mod indexes;
pub(crate) mod kernels;
pub(crate) mod lowering;
pub(crate) mod run;

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem;
use crate::domain::physical_plan::service::lowering::service::lowered;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{NodeReaders, PlanInput, PlanNodeId, Retention};

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
    /// The `hash_index` kernel.
    HashIndex => crate::domain::physical_plan::collections::kernels::hash_index::item::HashIndexItem,
    /// The `distinct` kernel.
    Distinct => crate::domain::physical_plan::collections::kernels::distinct::item::DistinctItem,
    /// The `group` kernel.
    Group => crate::domain::physical_plan::collections::kernels::group::item::GroupItem,
    /// The `text_map` kernel.
    TextMap => crate::domain::physical_plan::collections::kernels::text_map::item::TextMapItem,
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
/// `WrongOperation` when a node reads a later node.
pub(crate) fn plan_with(
    program: &LogicalProgram<'_>,
    rewrites: &[&dyn RewritesItem],
) -> Result<PhysicalPlan, PortableOperatorFault> {
    program.check_shape()?;
    let logical = program.nodes();
    let mut nodes: Vec<PhysicalNode> = logical
        .iter()
        .map(|node| PhysicalNode {
            lowering: lowered(node.operation, node.inputs.0, node.bindings.0),
            readers: NodeReaders(0),
            retained: Retention::Released,
        })
        .collect();
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
            node.retained = Retention::Retained;
        }
    }
    let mut indexes = Vec::new();
    for rewrite in rewrites {
        rewrite.rewrite(&mut nodes, &mut indexes);
    }
    // After the rewrites: a rewrite changes a probe's right rows, key and mode, and replaces probes.
    indexes::service::shared_indexes(&mut nodes, &mut indexes);
    Ok(PhysicalPlan { nodes, indexes })
}

/// The node at `id`, to change.
fn place(nodes: &mut [PhysicalNode], id: PlanNodeId) -> Option<&mut PhysicalNode> {
    nodes.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
}
