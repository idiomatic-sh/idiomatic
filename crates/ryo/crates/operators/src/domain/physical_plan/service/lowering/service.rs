//! The planner's one dispatch over the operation kinds, each arm its kind's one lowering: what a node reads, and the kernels
//! it runs.
use crate::domain::aggregate::types::GroupMeasure;
use crate::domain::ordered_dfs::types::DfsShape;
use crate::domain::physical_plan::types::lowering::value::{BoundScan, Lowering};
use crate::domain::physical_plan::types::{PlanInput, ProbeIndex, RowMap, RowSource};
use crate::domain::set_rows::types::{SetKind, SetLowering};
use crate::domain::worklist::types::WorklistShape;
use crate::domain::{aggregate, closure, cycles, ordered_dfs, worklist};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

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
        PlanInput::Frontier(place) => Some(RowSource::Frontier(*place)),
        PlanInput::Observation(place) => Some(RowSource::Observation(*place)),
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
            group: aggregate::service::lower(
                keys,
                distinct.map_or(GroupMeasure::Rows, GroupMeasure::DistinctValues),
            ),
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
            edge_index: None,
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
                    edge_index: None,
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
        Operation::Worklist {
            root,
            state,
            source,
            destination,
            order,
            before,
            after,
            lower,
            upper,
            root_keys,
            edge_keys,
            ..
        } => {
            let third = inputs.get(2).and_then(source_of);
            both()
                .zip(third)
                .map(|((roots, edges), lattice)| Lowering::Worklist {
                    roots,
                    edges,
                    lattice,
                    worklist: worklist::service::lower(WorklistShape {
                        root: *root,
                        state: *state,
                        edge_columns: [*source, *destination, *order, *before, *after],
                        lattice_columns: [*lower, *upper],
                        root_keys,
                        edge_keys,
                    }),
                })
        }
        Operation::Cycles {
            source,
            destination,
            ..
        } => first.map(|edges| Lowering::Components {
            edges,
            cycles: cycles::service::lower(*source, *destination),
        }),
        Operation::Rows { parameter, keys } => Some(Lowering::ScanBound {
            scan: BoundScan::Rows {
                parameter: *parameter,
                keys: keys.clone(),
                bindings: bindings.to_vec(),
            },
        }),
        Operation::Hosted { relation } => Some(Lowering::ScanBound {
            scan: BoundScan::Hosted(*relation),
        }),
    };
    lowering.unwrap_or(Lowering::Unlowered)
}
