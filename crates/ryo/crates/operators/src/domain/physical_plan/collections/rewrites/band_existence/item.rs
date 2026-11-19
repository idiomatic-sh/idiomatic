//! Spike (ryo-fmx3): the `band_existence` item of the `rewrites` collection. A distinct projection of left columns only, over
//! a select whose every comparison sets one right column against a left column, over a plain inner equality probe, keeps a
//! left row's projection exactly when an equal-keyed right row lies within the band its left columns set. So the projection
//! runs as an `ordered_match` band semi match over the probe's two sources (sorted right orders, one search per left row),
//! and the probe and the select lose it as a reader: the pairs are never formed.

use super::super::interface::RewritesItem;
use super::super::ordered_existence::item::{NumberProof, numbered};
use crate::domain::equality_join::types::{ProbeKey, ProbeMode, ProbeParts};
use crate::domain::ordered_match::types::{Band, BandBound, OrderedParts};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{EngineUses, KeyedIndex, NodeReaders, PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Operand, RepeatedRows, Side, Test};

/// `band_existence`, one item of the `rewrites` collection.
pub(crate) struct BandExistenceItem;

/// A projection the item recognised and what it becomes.
struct Recognised {
    /// The projection.
    project: PlanNodeId,
    /// The select and the probe, which lose the projection as a reader.
    replaced: [PlanNodeId; 2],
    /// The band semi match it becomes.
    lowering: Lowering,
}

impl RewritesItem for BandExistenceItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>) {
        if std::env::var_os("RYO_NO_BAND").is_some() {
            return;
        }
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
            if let Some(node) = node_mut(nodes, recognised.project) {
                node.lowering = recognised.lowering;
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

/// The node `source` reads when only one node reads it and no rule keeps it.
fn only_read(nodes: &[PhysicalNode], source: RowSource) -> Option<(PlanNodeId, &Lowering)> {
    let RowSource::Node(id) = source else {
        return None;
    };
    let node = node_of(nodes, id)?;
    (node.readers == NodeReaders(1) && node.engine == EngineUses(0)).then_some((id, &node.lowering))
}

/// The band shape at `project`, or None.
fn recognised(
    nodes: &[PhysicalNode],
    indexes: &[KeyedIndex],
    project: PlanNodeId,
) -> Option<Recognised> {
    let Lowering::Project {
        source,
        project: parts,
    } = &node_of(nodes, project)?.lowering
    else {
        return None;
    };
    if parts.distinct != RepeatedRows::Dropped {
        return None;
    }
    let (select, Lowering::Filter { source, select: kept }) = only_read(nodes, *source)? else {
        return None;
    };
    if !kept.kept.is_empty() || !kept.bindings.is_empty() || kept.comparisons.is_empty() {
        return None;
    }
    let (
        join,
        Lowering::Probe {
            left,
            right,
            probe:
                ProbeParts {
                    mode: ProbeMode::Inner,
                    key: ProbeKey::Columns {
                        left: left_on,
                        right: right_on,
                    },
                    filter,
                    output,
                    ..
                },
            ..
        },
    ) = only_read(nodes, *source)?
    else {
        return None;
    };
    if !filter.comparisons.is_empty() || !filter.kept.is_empty() || left_on.len() != right_on.len() {
        return None;
    }
    let at = |column: ColumnIndex| output.get(usize::from(column)).copied();
    // Every projected column is a left column.
    let projected: Vec<JoinColumn> = parts
        .columns
        .iter()
        .map(|column| at(*column).filter(|joined| joined.side == Side::Left))
        .collect::<Option<_>>()?;
    // Every comparison sets the one right column against a left column.
    let mut order: Option<ColumnIndex> = None;
    let mut bounds = Vec::new();
    for comparison in &kept.comparisons {
        let (Operand::Column(one), Operand::Column(other)) = (&comparison.left, &comparison.right)
        else {
            return None;
        };
        let (one, other) = (at(*one)?, at(*other)?);
        // Normalised to `right TEST left`.
        let (right_column, left_column, test) = match (one.side, other.side) {
            (Side::Right, Side::Left) => (one.column, other.column, comparison.test),
            (Side::Left, Side::Right) => (
                other.column,
                one.column,
                match comparison.test {
                    Test::LessThan => Test::GreaterThan,
                    Test::LessThanOrEqual => Test::GreaterThanOrEqual,
                    Test::GreaterThan => Test::LessThan,
                    Test::GreaterThanOrEqual => Test::LessThanOrEqual,
                    _ => return None,
                },
            ),
            _ => return None,
        };
        if *order.get_or_insert(right_column) != right_column {
            return None;
        }
        let (above, inclusive) = match test {
            Test::GreaterThan => (true, false),
            Test::GreaterThanOrEqual => (true, true),
            Test::LessThan => (false, false),
            Test::LessThanOrEqual => (false, true),
            _ => return None,
        };
        bounds.push(BandBound {
            column: left_column,
            above,
            inclusive,
        });
    }
    let order = order?;
    if numbered(nodes, indexes, *right, order, join) != NumberProof::Proven
        || bounds
            .iter()
            .any(|bound| numbered(nodes, indexes, *left, bound.column, join) != NumberProof::Proven)
    {
        return None;
    }
    Some(Recognised {
        project,
        replaced: [select, join],
        lowering: Lowering::OrderedMatch {
            left: *left,
            right: *right,
            ordered: OrderedParts {
                left_on: left_on.clone(),
                right_on: right_on.clone(),
                left_order: bounds.first()?.column,
                right_order: order,
                output: projected,
                band: Some(Band { bounds }),
            },
        },
    })
}
