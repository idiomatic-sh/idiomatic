//! The planner's one dispatch over the operation kinds, each kind lowered once by its own concept; the registry of the rewrites
//! it applies over what it lowered, and the `kernels!` catalog of the kernels the lowerings run.
pub(crate) mod kernels;
pub(crate) mod run;

use crate::domain::aggregate::types::GroupMeasure;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::ordered_dfs::types::DfsShape;
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    NodeReaders, PlanInput, PlanNodeId, Retention, RowSource, ServedAt, Unserved,
};
use crate::domain::set_rows::types::{SetKind, SetLowering};
use crate::domain::{aggregate, closure, cycles, ordered_dfs};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;

/// Every rewrite, in the order the planner applies them: each an equivalence, so a plan answers the same rows with or without
/// it.
pub(crate) const REWRITES: &[&dyn RewritesItem] = &[
    &crate::domain::physical_plan::collections::rewrites::keyed_select::item::KeyedSelectItem,
    &crate::domain::physical_plan::collections::rewrites::ordered_existence::item::OrderedExistenceItem,
    &crate::domain::physical_plan::collections::rewrites::existence_probe::item::ExistenceProbeItem,
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
    /// The `reach` kernel.
    Reach => crate::domain::physical_plan::collections::kernels::reach::item::ReachItem,
    /// The `directed_walk` kernel.
    DirectedWalk => crate::domain::physical_plan::collections::kernels::directed_walk::item::DirectedWalkItem,
    /// The `ordered_walk` kernel.
    OrderedWalk => crate::domain::physical_plan::collections::kernels::ordered_walk::item::OrderedWalkItem,
    /// The `components` kernel.
    Components => crate::domain::physical_plan::collections::kernels::components::item::ComponentsItem,
}

/// The physical plan of `program`: each node lowered once, every registered rewrite applied, and every node of a kind of a
/// later stage recorded as unserved.
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
    let mut unserved = Vec::new();
    for ((node, physical), id) in logical.iter().zip(&nodes).zip((0..).map(PlanNodeId)) {
        if physical.lowering == Lowering::Unlowered {
            unserved.push(Unserved {
                node: id,
                until: stage_of(node.operation),
            });
        }
    }
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
    Ok(PhysicalPlan {
        nodes,
        indexes,
        unserved,
    })
}

/// The node at `id`, to change.
fn place(nodes: &mut [PhysicalNode], id: PlanNodeId) -> Option<&mut PhysicalNode> {
    nodes.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// What a node reads its rows from, when this stage's kernels read it.
fn source_of(input: &PlanInput) -> Option<RowSource> {
    match input {
        PlanInput::Node(node) => Some(RowSource::Node(*node)),
        PlanInput::Relation(relation) => Some(
            match crate::domain::tree_cells::service::native_form(*relation) {
                Some(_) => RowSource::Tree(*relation),
                None => RowSource::Facts(*relation),
            },
        ),
        PlanInput::Bound(_) | PlanInput::Observation(_) => None,
    }
}

/// The kernels of one operation: the planner's one dispatch over the kinds, each arm its kind's one lowering.
pub(crate) fn lowered(
    operation: &Operation,
    inputs: &[PlanInput],
    bindings: &[ParameterBinding],
) -> Lowering {
    let first = inputs.first().and_then(source_of);
    let both = || first.zip(inputs.get(1).and_then(source_of));
    let set = |kind: SetKind, left_on, right_on| {
        both().map(|(left, right)| {
            match crate::domain::set_rows::service::lower(kind, left_on, right_on) {
                SetLowering::Distinct => Lowering::Distinct {
                    sources: [left, right],
                },
                SetLowering::Probe(probe) => Lowering::Probe { left, right, probe },
            }
        })
    };
    let lowering = match operation {
        Operation::Select {
            comparisons, kept, ..
        } => first.map(|source| Lowering::Filter {
            source,
            select: crate::domain::select::service::lower(comparisons, kept, bindings),
        }),
        Operation::Project {
            columns, distinct, ..
        } => first.map(|source| Lowering::Project {
            source,
            project: crate::domain::project::service::lower(columns, *distinct),
        }),
        Operation::Extend { values, .. } => first.map(|source| Lowering::Extend {
            source,
            extension: crate::domain::append::service::lower(values, bindings),
        }),
        Operation::Join {
            left_on,
            right_on,
            output,
            filter,
            view,
            reach: None,
            ..
        } => both().map(|(left, right)| Lowering::Probe {
            left,
            right,
            probe: crate::domain::equality_join::service::lower(
                crate::domain::equality_join::types::JoinShape {
                    left_on,
                    right_on,
                    output,
                    view,
                    reach: None,
                },
                filter,
                bindings,
            ),
        }),
        Operation::Union { .. } => set(SetKind::Union, &[], &[]),
        Operation::Intersection { .. } => set(SetKind::Intersection, &[], &[]),
        Operation::Difference { .. } => set(SetKind::Difference, &[], &[]),
        Operation::Antijoin {
            left_on, right_on, ..
        } => set(SetKind::Antijoin, left_on, right_on),
        Operation::GroupCount { keys, distinct, .. } => first.map(|source| Lowering::Group {
            source,
            group: aggregate::service::lower(
                keys,
                distinct.map_or(GroupMeasure::Rows, GroupMeasure::DistinctValues),
            ),
        }),
        Operation::Join {
            left_on,
            right_on,
            output,
            filter,
            view,
            reach: Some(reach),
            ..
        } => both().map(|(left, edges)| Lowering::SeededReach {
            left,
            edges,
            graph: closure::service::lower(&reach.keys, reach.source, reach.destination),
            join: crate::domain::equality_join::service::seeded(
                crate::domain::equality_join::types::JoinShape {
                    left_on,
                    right_on,
                    output,
                    view,
                    reach: Some(reach),
                },
                filter,
                bindings,
            ),
        }),
        Operation::GroupDigest { keys, over, .. } => first.map(|source| Lowering::Group {
            source,
            group: aggregate::service::lower(keys, GroupMeasure::Digest(*over)),
        }),
        Operation::Closure {
            keys,
            source,
            destination,
            ..
        } => first.map(|edges| Lowering::Reach {
            edges,
            graph: closure::service::lower(keys, *source, *destination),
        }),
        Operation::Axis {
            direction,
            id,
            parent,
            child,
            domain_id,
            domain_filter,
            domain_view,
            output,
            ..
        } => {
            let third = inputs.get(2).and_then(source_of);
            both()
                .zip(third)
                .map(|((starts, edges), domain)| Lowering::Axis {
                    starts,
                    edges,
                    domain,
                    axis: crate::domain::axis::service::lower(
                        crate::domain::axis::types::AxisShape {
                            direction: *direction,
                            id: *id,
                            parent: *parent,
                            child: *child,
                            domain_id: *domain_id,
                            output,
                        },
                        domain_filter,
                        domain_view,
                        bindings,
                    ),
                })
        }
        Operation::OrderedDfs {
            root,
            source,
            destination,
            order,
            edge,
            root_keys,
            edge_keys,
            ..
        } => both().map(|(roots, edges)| Lowering::OrderedWalk {
            roots,
            edges,
            dfs: ordered_dfs::service::lower(DfsShape {
                root: *root,
                edge_columns: [*source, *destination, *order, *edge],
                root_keys,
                edge_keys,
            }),
        }),
        Operation::Cycles {
            source,
            destination,
            ..
        } => first.map(|edges| Lowering::Components {
            edges,
            cycles: cycles::service::lower(*source, *destination),
        }),
        Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    };
    lowering.unwrap_or(Lowering::Unlowered)
}

/// The stage of the swap whose kernels lower a kind: this stage's kinds are `ServedAt(1)`.
pub(crate) fn stage_of_kind(kind: OperationKind) -> ServedAt {
    ServedAt(match kind {
        OperationKind::Select
        | OperationKind::Project
        | OperationKind::Extend
        | OperationKind::Join
        | OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin
        | OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::OrderedDfs
        | OperationKind::Cycles => 1,
        OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Replace
        | OperationKind::OrderedMatch
        | OperationKind::Words
        | OperationKind::SpanBetween
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered
        | OperationKind::SpanBounds => 2,
        OperationKind::Rows | OperationKind::Hosted => 4,
    })
}

/// The stage that lowers a node this stage does not: its kind's, or for one of this stage's kinds, the stage that reads what
/// it reads: a bound table or an observation at the fourth.
fn stage_of(operation: &Operation) -> ServedAt {
    match (stage_of_kind(kind_of(operation)), operation) {
        (ServedAt(1), _) => ServedAt(4),
        (stage, _) => stage,
    }
}
