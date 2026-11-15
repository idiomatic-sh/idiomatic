//! The `ordered_existence` item of the `rewrites` collection: an antijoin whose right rows are the distinct projection of a
//! strict-order select over a plain self join of one source, partitioned by the join's keys, keeps a left row exactly when
//! no row of its partition orders past it. So the antijoin probes the source itself in `ProbeMode::Ordered`, deciding by
//! each partition's extremum, and the self join, the select and the projection lose it as a reader. It applies only where
//! the order column is proven a number from the relations it is read from, and where no other node reads the three.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{
    EmptyRight, OrderedProbe, PartitionExtremum, ProbeKey, ProbeMode, ProbeParts,
};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{
    KeyedIndex, NodeReaders, PlanNodeId, Retention, RowSource,
};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
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
    /// The self join, the select over it and the distinct projection of the select, which lose the antijoin as a reader.
    replaced: [PlanNodeId; 3],
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
                lowering: Lowering::Probe { right, probe, .. },
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
    (node.readers == NodeReaders(1) && node.retained == Retention::Released).then_some(OnlyRead {
        id,
        lowering: &node.lowering,
    })
}

/// Whether a column is proven to hold a number on every row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumberProof {
    /// Every row holds a number there.
    Proven,
    /// No proof was found.
    Unproven,
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
    let Lowering::Probe {
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
    } = &node_of(nodes, antijoin)?.lowering
    else {
        return None;
    };
    if !filter.comparisons.is_empty() {
        return None;
    }
    let OnlyRead {
        id: project,
        lowering: Lowering::Project {
            source,
            project: parts,
        },
    } = only_read(nodes, *right)?
    else {
        return None;
    };
    if parts.distinct != RepeatedRows::Dropped {
        return None;
    }
    let OnlyRead {
        id: select,
        lowering: Lowering::Filter {
            source,
            select: kept,
        },
    } = only_read(nodes, *source)?
    else {
        return None;
    };
    let ([comparison], true) = (kept.comparisons.as_slice(), kept.kept.is_empty()) else {
        return None;
    };
    let OnlyRead {
        id: join,
        lowering:
            Lowering::Probe {
                left: source,
                right: other,
                probe: joined,
            },
    } = only_read(nodes, *source)?
    else {
        return None;
    };
    let ProbeParts {
        mode: ProbeMode::Inner,
        key: ProbeKey::Columns {
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
    if source != other || source != left || partition != right_on || !filter.comparisons.is_empty()
    {
        return None;
    }
    let (Operand::Column(one), Operand::Column(another)) = (&comparison.left, &comparison.right)
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
    let order = one.column;
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
    (numbered(nodes, indexes, *source, order, antijoin) == NumberProof::Proven).then(|| {
        Recognised {
            antijoin,
            source: *source,
            replaced: [join, select, project],
            partition: partition.clone(),
            order: OrderedProbe {
                left_order: order,
                right_order: order,
                extremum,
            },
        }
    })
}

/// Whether every row `source` answers holds a number in `column`, proven from the relations it is read from through
/// value-preserving kernels only: a filter, a lookup, a projection, a probe and a distinct union. No other kernel proves it.
fn numbered(
    nodes: &[PhysicalNode],
    indexes: &[KeyedIndex],
    source: RowSource,
    column: ColumnIndex,
    before: PlanNodeId,
) -> NumberProof {
    let stored = |relation, column: ColumnIndex| match kinds(relation).get(usize::from(column)) {
        Some(CellKind::Number) => NumberProof::Proven,
        _ => NumberProof::Unproven,
    };
    let kept = |kept: &[ColumnIndex], column: ColumnIndex| {
        if kept.is_empty() {
            Some(column)
        } else {
            kept.get(usize::from(column)).copied()
        }
    };
    let id = match source {
        RowSource::Facts(relation) | RowSource::Tree(relation) => return stored(relation, column),
        RowSource::Node(id) if id.0 < before.0 => id,
        RowSource::Node(_) => return NumberProof::Unproven,
    };
    let Some(node) = node_of(nodes, id) else {
        return NumberProof::Unproven;
    };
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the proof follows only the value-preserving kernels; every other kernel declines it"
    )]
    match &node.lowering {
        Lowering::Filter { source, select } => kept(&select.kept, column)
            .map_or(NumberProof::Unproven, |column| {
                numbered(nodes, indexes, *source, column, id)
            }),
        Lowering::Lookup { index, select, .. } => kept(&select.kept, column)
            .zip(indexes.get(index.0))
            .map_or(NumberProof::Unproven, |(column, index)| {
                stored(index.relation, column)
            }),
        Lowering::Project { source, project } => project
            .columns
            .get(usize::from(column))
            .map_or(NumberProof::Unproven, |column| {
                numbered(nodes, indexes, *source, *column, id)
            }),
        Lowering::Probe { left, right, probe } => match probe.mode {
            ProbeMode::Inner => {
                probe
                    .output
                    .get(usize::from(column))
                    .map_or(NumberProof::Unproven, |output| {
                        let side = if output.side == Side::Left {
                            *left
                        } else {
                            *right
                        };
                        numbered(nodes, indexes, side, output.column, id)
                    })
            }
            ProbeMode::Semi | ProbeMode::Anti | ProbeMode::Ordered(_) => {
                numbered(nodes, indexes, *left, column, id)
            }
        },
        Lowering::Distinct { sources } => sources
            .iter()
            .map(|source| numbered(nodes, indexes, *source, column, id))
            .find(|proof| *proof == NumberProof::Unproven)
            .unwrap_or(NumberProof::Proven),
        _ => NumberProof::Unproven,
    }
}
