//! The planner's one dispatch over the operation kinds, each kind lowered once by its own concept; the registry of the rewrites
//! it applies over what it lowered, and the `kernels!` catalog of the kernels the lowerings run.
pub(crate) mod indexes;
pub(crate) mod run;

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    Lowering, NodeReaders, PhysicalNode, PlanInput, PlanNodeId, ProbeIndex, Retention, RowMap,
    RowSource, ServedAt, Unserved,
};
use crate::domain::set_rows::types::{SetKind, SetLowering};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;

/// Every rewrite, in the order the planner applies them: each an equivalence, so a plan answers the same rows with or without
/// it.
pub(crate) const REWRITES: &[&dyn RewritesItem] = &[
    &crate::domain::physical_plan::collections::rewrites::keyed_select::item::KeyedSelectItem,
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
    /// The `text_map` kernel.
    TextMap => crate::domain::physical_plan::collections::kernels::text_map::item::TextMapItem,
    /// The `ordered_match` kernel.
    OrderedMatch => crate::domain::physical_plan::collections::kernels::ordered_match::item::OrderedMatchItem,
    /// The `render` kernel.
    Render => crate::domain::physical_plan::collections::kernels::render::item::RenderItem,
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
    indexes::service::shared_indexes(&mut nodes, &mut indexes);
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
                SetLowering::Probe(probe) => Lowering::Probe {
                    left,
                    right,
                    probe,
                    index: ProbeIndex::Own,
                },
            }
        })
    };
    let map = |map: RowMap| first.map(|source| Lowering::TextMap { source, map });
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
            index: ProbeIndex::Own,
        }),
        Operation::Union { .. } => set(SetKind::Union, &[], &[]),
        Operation::Intersection { .. } => set(SetKind::Intersection, &[], &[]),
        Operation::Difference { .. } => set(SetKind::Difference, &[], &[]),
        Operation::Antijoin {
            left_on, right_on, ..
        } => set(SetKind::Antijoin, left_on, right_on),
        Operation::GroupCount { keys, distinct, .. } => first.map(|source| Lowering::Group {
            source,
            group: crate::domain::aggregate::service::lower(keys, *distinct),
        }),
        Operation::Capture { path, pattern, .. } => map(RowMap::Capture(
            crate::domain::capture::service::lower(*path, pattern),
        )),
        Operation::Rank { column, sets, .. } => map(RowMap::Rank(
            crate::domain::rank::service::lower(*column, *sets, bindings),
        )),
        Operation::Replace {
            text,
            pattern,
            replacement,
            ..
        } => map(RowMap::Replace(crate::domain::replace::service::lower(
            *text,
            pattern,
            replacement,
        ))),
        Operation::Words { text, .. } => {
            map(RowMap::Words(crate::domain::words::service::lower(*text)))
        }
        Operation::SpanBetween { start, end, .. } => map(RowMap::SpanBetween(
            crate::domain::span_between::service::lower(*start, *end),
        )),
        Operation::SourceMatches {
            span,
            base_line,
            pattern,
            capture,
            capture_text,
            capture_line,
            ..
        } => map(RowMap::SourceMatches(
            crate::domain::source_matches::service::lower(
                *span,
                *base_line,
                pattern,
                capture.as_ref(),
                *capture_text,
                *capture_line,
            ),
        )),
        Operation::FitsRendered {
            texts,
            literals,
            span,
            max_line,
            tab_width,
            ..
        } => map(RowMap::FitsRendered(
            crate::domain::fits_rendered::service::lower(
                texts, literals, *span, *max_line, *tab_width,
            ),
        )),
        Operation::SpanBounds { span, .. } => map(RowMap::SpanBounds(
            crate::domain::span_bounds::service::lower(*span),
        )),
        Operation::OrderedMatch {
            left_on,
            right_on,
            left_order,
            right_order,
            output,
            ..
        } => both().map(|(left, right)| Lowering::OrderedMatch {
            left,
            right,
            ordered: crate::domain::ordered_match::service::lower(
                left_on,
                right_on,
                *left_order,
                *right_order,
                output,
            ),
        }),
        Operation::RenderSyntax { node, context, .. } => first.map(|source| Lowering::Render {
            source,
            render: crate::domain::render_batch::service::lower(*node, *context),
        }),
        Operation::Join { reach: Some(_), .. }
        | Operation::GroupDigest { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    };
    lowering.unwrap_or(Lowering::Unlowered)
}

/// The stage of the swap whose kernels lower a kind: the first stage's kinds are `ServedAt(1)`, the text kinds `ServedAt(2)`.
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
        | OperationKind::GroupCount => 1,
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
        OperationKind::GroupDigest
        | OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::OrderedDfs
        | OperationKind::Cycles => 3,
        OperationKind::Rows | OperationKind::Hosted => 4,
    })
}

/// The stage that lowers a node this branch does not: its kind's, or for a kind of a served stage, the stage that reads what
/// it reads: a join's closure demand at the third, a bound table or an observation at the fourth.
fn stage_of(operation: &Operation) -> ServedAt {
    match (stage_of_kind(kind_of(operation)), operation) {
        (_, Operation::Join { reach: Some(_), .. }) => ServedAt(3),
        (stage, _) if SERVED.contains(&stage) => ServedAt(4),
        (stage, _) => stage,
    }
}

/// The stages whose kinds this branch lowers: a node of one of their kinds that is not lowered reads what a later stage
/// serves. Branch only: deleted with `ServedAt`.
pub(crate) const SERVED: &[ServedAt] = &[ServedAt(1), ServedAt(2)];
