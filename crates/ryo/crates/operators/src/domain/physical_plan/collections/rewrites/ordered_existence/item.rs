//! The `ordered_existence` item of the `rewrites` collection: an antijoin whose right rows are the distinct projection of a
//! strict-order select over a plain self join of one source, partitioned by the join's keys, keeps a left row exactly when
//! no row of its partition orders past it. So the antijoin probes the source itself in `ProbeMode::Ordered`, deciding by
//! each partition's extremum, and the self join, the select and the projection lose it as a reader. It applies only where
//! the order column is proven a number from the relations it is read from, and where no other node reads the three.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{
    BandParts, EmptyRight, OrderedProbe, PartitionExtremum, ProbeKey, ProbeMode, ProbeParts,
};
use crate::domain::physical_plan::service::numbers::service::numbered;
use crate::domain::physical_plan::service::numbers::types::NumberProof;
use crate::domain::physical_plan::types::lowering::value::{
    BandJoinNode, FilterNode, Lowering, ProbeNode, ProjectNode,
};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{
    EngineUses, KeyedIndex, NodeReaders, PlanNodeId, RowSource,
};
use ryo_values::domain::band_joins::types::BandTest;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, JoinColumn, Operand, RepeatedRows, Side, Test,
};

/// `ordered_existence`, one item of the `rewrites` collection.
pub(crate) struct OrderedExistenceItem;

/// An antijoin the item recognised: its node, the source its self join reads, the nodes it stops reading, and the probe
/// it becomes.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Recognised {
    /// The antijoin.
    antijoin: PlanNodeId,
    /// The source both sides of the self join read, and the antijoin's left rows.
    source: RowSource,
    /// The self join, the select over it when the order is a filter of the join, and the distinct projection, which lose
    /// the antijoin as a reader.
    replaced: Vec<PlanNodeId>,
    /// The partition columns of the source.
    partition: Vec<ColumnIndex>,
    /// The order the probe compares by.
    order: OrderedProbe,
}

impl RewritesItem for OrderedExistenceItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>) {
        let found: Vec<Recognised> = (0..nodes.len())
            .filter_map(|at| {
                recognised(
                    nodes,
                    indexes,
                    PlanNodeId(u32::try_from(at).unwrap_or(u32::MAX)),
                )
            })
            .collect();
        for recognised in found {
            for replaced in recognised.replaced {
                if let Some(node) = node_mut(nodes, replaced) {
                    node.readers = NodeReaders(0);
                }
            }
            // The self join read the source on both sides; the antijoin now reads it on its right as well as its left.
            if let RowSource::Node(source) = recognised.source
                && let Some(node) = node_mut(nodes, source)
            {
                node.readers = NodeReaders(node.readers.0.saturating_sub(1));
            }
            if let Some(PhysicalNode {
                lowering: Lowering::Probe(ProbeNode { right, probe, .. }),
                ..
            }) = node_mut(nodes, recognised.antijoin)
            {
                *right = recognised.source;
                probe.mode = ProbeMode::Ordered(recognised.order);
                probe.key = ProbeKey::Columns {
                    left: recognised.partition.clone(),
                    right: recognised.partition,
                };
            }
        }
    }
}

/// The node at `id`.
fn node_of(nodes: &[PhysicalNode], id: PlanNodeId) -> Option<&PhysicalNode> {
    nodes.get(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// The node at `id`, to change.
fn node_mut(nodes: &mut [PhysicalNode], id: PlanNodeId) -> Option<&mut PhysicalNode> {
    nodes.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// A node read by one reader only and kept by no rule: its id and its lowering.
struct OnlyRead<'a> {
    /// The node.
    id: PlanNodeId,
    /// Its kernels.
    lowering: &'a Lowering,
}

/// The node `source` reads when it reads one that no node but its one reader reads and no rule keeps.
fn only_read(nodes: &[PhysicalNode], source: RowSource) -> Option<OnlyRead<'_>> {
    let RowSource::Node(id) = source else {
        return None;
    };
    let node = node_of(nodes, id)?;
    (node.readers == NodeReaders(1) && node.engine == EngineUses(0)).then_some(OnlyRead {
        id,
        lowering: &node.lowering,
    })
}

/// The ordered existence shape at `antijoin`: an antijoin of a source `S` with the distinct projection of S's left columns
/// of a strict-order select over a plain self join of S on its partition, where the antijoin's left rows are S itself and
/// its key maps each of its columns to the projected column it came from, covering the partition and the order. Then a
/// row of S has an equal-keyed projected row exactly when a row of its partition orders strictly past it, and the
/// antijoin keeps a row exactly when its partition's extremum does not pass it. None for any other shape.
fn recognised(
    nodes: &[PhysicalNode],
    indexes: &[KeyedIndex],
    antijoin: PlanNodeId,
) -> Option<Recognised> {
    let Lowering::Probe(ProbeNode {
        left,
        right,
        probe:
            ProbeParts {
                mode: ProbeMode::Anti,
                key:
                    ProbeKey::Columns {
                        left: anti_left,
                        right: anti_right,
                    },
                filter,
                repeated: RepeatedRows::Kept,
                empty_right: EmptyRight::EveryLeftRow,
                ..
            },
        ..
    }) = &node_of(nodes, antijoin)?.lowering
    else {
        return None;
    };
    if !filter.comparisons.is_empty() {
        return None;
    }
    let OnlyRead {
        id: project,
        lowering:
            Lowering::Project(ProjectNode {
                source,
                project: parts,
            }),
    } = only_read(nodes, *right)?
    else {
        return None;
    };
    if parts.distinct != RepeatedRows::Dropped {
        return None;
    }
    let Paired {
        replaced,
        partition,
        order,
        extremum,
        output,
    } = self_ordered(nodes, *source, *left)?;
    // The antijoin's key maps each left column to the projected left column it came from.
    let projected: Vec<ColumnIndex> = parts
        .columns
        .iter()
        .map(|column| match output.get(usize::from(*column))? {
            JoinColumn {
                side: Side::Left,
                column,
            } => Some(*column),
            JoinColumn {
                side: Side::Right, ..
            } => None,
        })
        .collect::<Option<_>>()?;
    let mapped = anti_left.len() == anti_right.len()
        && anti_left
            .iter()
            .zip(anti_right)
            .all(|(left_column, right_column)| {
                projected.get(usize::from(*right_column)) == Some(left_column)
            });
    if !mapped
        || !partition
            .iter()
            .chain([&order])
            .all(|column| anti_left.contains(column))
    {
        return None;
    }
    (numbered(nodes, indexes, *left, order, antijoin) == NumberProof::Proven).then(|| {
        let mut replaced = replaced;
        replaced.push(project);
        Recognised {
            antijoin,
            source: *left,
            replaced,
            partition: partition.to_vec(),
            order: OrderedProbe {
                left_order: order,
                right_order: order,
                extremum,
            },
        }
    })
}

/// A self join of one source that pairs each row with the rows of its partition strictly past it in one order: the nodes that
/// make the pairs, the partition, the order and which end of it is never passed, and the columns of the pairs.
struct Paired<'a> {
    /// The self join, and the select of its order when the order is a filter of it.
    replaced: Vec<PlanNodeId>,
    /// The partition columns of the source.
    partition: &'a [ColumnIndex],
    /// The order column of the source.
    order: ColumnIndex,
    /// The end of the order no row of a partition passes.
    extremum: PartitionExtremum,
    /// The columns of the pairs, each of one side.
    output: &'a [JoinColumn],
}

/// The pairs `pairs` reads when they pair each row of `rows` with the rows of its partition strictly past it in one order,
/// each one node read only by its reader: an inner probe of `rows` with itself on its partition with a select of one strict
/// order comparison of one column of each side over it, or the band join of `rows` with itself on its partition whose one
/// strict edge compares one column of each side, as the band lowering writes the same join.
fn self_ordered(nodes: &[PhysicalNode], pairs: RowSource, rows: RowSource) -> Option<Paired<'_>> {
    let read = only_read(nodes, pairs)?;
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the pairs are a band join or a select over a probe; every other kernel is no such shape"
    )]
    match read.lowering {
        Lowering::BandJoin(BandJoinNode {
            left,
            right,
            band:
                BandParts {
                    left_on,
                    right_on,
                    band,
                    output,
                    ..
                },
        }) => {
            let [edge] = band.as_slice() else {
                return None;
            };
            let extremum = match edge.test {
                BandTest::Above => PartitionExtremum::Maximum,
                BandTest::Below => PartitionExtremum::Minimum,
                BandTest::AtOrAbove | BandTest::AtOrBelow => return None,
            };
            (*left == rows && *right == rows && left_on == right_on && edge.left == edge.right)
                .then_some(Paired {
                    replaced: vec![read.id],
                    partition: left_on,
                    order: edge.left,
                    extremum,
                    output,
                })
        }
        Lowering::Filter(FilterNode {
            source,
            select: kept,
        }) => {
            let ([comparison], true) = (kept.comparisons.as_slice(), kept.kept.is_empty()) else {
                return None;
            };
            let OnlyRead {
                id: join,
                lowering:
                    Lowering::Probe(ProbeNode {
                        left: source,
                        right: other,
                        probe: joined,
                        ..
                    }),
            } = only_read(nodes, *source)?
            else {
                return None;
            };
            let ProbeParts {
                mode: ProbeMode::Inner(_),
                key:
                    ProbeKey::Columns {
                        left: partition,
                        right: right_on,
                    },
                filter,
                output,
                ..
            } = joined
            else {
                return None;
            };
            // A plain self join of one source on its partition, whose rows are the antijoin's left rows.
            if *source != *other
                || *source != rows
                || partition != right_on
                || !filter.comparisons.is_empty()
            {
                return None;
            }
            let (Operand::Column(one), Operand::Column(another)) =
                (&comparison.left, &comparison.right)
            else {
                return None;
            };
            let one = output.get(usize::from(*one))?;
            let another = output.get(usize::from(*another))?;
            if one.column != another.column || one.side == another.side {
                return None;
            }
            let extremum = match (comparison.test, one.side) {
                (Test::LessThan, Side::Left) | (Test::GreaterThan, Side::Right) => {
                    PartitionExtremum::Maximum
                }
                (Test::GreaterThan, Side::Left) | (Test::LessThan, Side::Right) => {
                    PartitionExtremum::Minimum
                }
                _ => return None,
            };
            Some(Paired {
                replaced: vec![join, read.id],
                partition,
                order: one.column,
                extremum,
                output,
            })
        }
        _ => None,
    }
}
