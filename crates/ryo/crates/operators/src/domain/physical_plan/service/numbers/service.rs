//! Whether a column of a node's rows holds a number on every row, proven from the relations it is read from through the
//! value-preserving kernels only.
use super::types::NumberProof;
use crate::domain::equality_join::types::ProbeMode;
use crate::domain::physical_plan::types::lowering::value::{
    DistinctNode, FilterNode, LookupNode, Lowering, ProbeNode, ProjectNode,
};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{KeyedIndex, PlanNodeId, RowSource};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side};

/// Whether every row `source` answers holds a number in `column`, proven from the relations it is read from through
/// value-preserving kernels only: a filter, a lookup, a projection, a probe and a distinct union. No other kernel proves it.
pub(crate) fn numbered(
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
        RowSource::Node(_) | RowSource::Frontier(_) | RowSource::Observation(_) => {
            return NumberProof::Unproven;
        }
    };
    let Some(node) = nodes.get(usize::try_from(id.0).unwrap_or(usize::MAX)) else {
        return NumberProof::Unproven;
    };
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the proof follows only the value-preserving kernels; every other kernel declines it"
    )]
    match &node.lowering {
        Lowering::Filter(FilterNode { source, select }) => kept(&select.kept, column)
            .map_or(NumberProof::Unproven, |column| {
                numbered(nodes, indexes, *source, column, id)
            }),
        Lowering::Lookup(LookupNode { index, select, .. }) => kept(&select.kept, column)
            .zip(indexes.get(index.0))
            .map_or(NumberProof::Unproven, |(column, index)| {
                match index.source {
                    RowSource::Facts(relation) => stored(relation, column),
                    RowSource::Node(_)
                    | RowSource::Tree(_)
                    | RowSource::Frontier(_)
                    | RowSource::Observation(_) => NumberProof::Unproven,
                }
            }),
        Lowering::Project(ProjectNode { source, project }) => project
            .columns
            .get(usize::from(column))
            .map_or(NumberProof::Unproven, |column| {
                numbered(nodes, indexes, *source, *column, id)
            }),
        Lowering::Probe(ProbeNode {
            left, right, probe, ..
        }) => match probe.mode {
            ProbeMode::Inner(_) => {
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
        Lowering::Distinct(DistinctNode { sources }) => sources
            .iter()
            .map(|source| numbered(nodes, indexes, *source, column, id))
            .find(|proof| *proof == NumberProof::Unproven)
            .unwrap_or(NumberProof::Proven),
        _ => NumberProof::Unproven,
    }
}
