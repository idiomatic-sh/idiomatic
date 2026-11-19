//! Bind the lowered right key, output order and the entity that uniquely determines its peer.

use crate::domain::entity_queries::types::{
    AncestorDistance, EntityAttribute as Field, EntityColumn, EntityExportId,
    EntityQueryOp as Operation, EntityStage as Stage,
};
use crate::domain::equality_join::types::{EmptyRight, Existence, ProbeKey, ProbeMode, ProbeParts};
use crate::domain::physical_plan::types::{
    IndexKey, KeyedIndex, PlanNodeId, ProbeIndex, RowSource,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows, Side, StepId};

pub(in super::super) fn probe(
    probe: &ProbeParts,
    index: ProbeIndex,
    right: RowSource,
    indexes: &[KeyedIndex],
    operands: [StepId; 2],
    stages: &[Stage],
    physical: PlanNodeId,
) -> Option<Stage> {
    let ProbeKey::Columns {
        left: left_key,
        right: right_key,
    } = &probe.key
    else {
        return None;
    };
    if !probe.filter.comparisons.is_empty()
        || !probe.filter.kept.is_empty()
        || !probe.filter.bindings.is_empty()
        || probe.repeated != RepeatedRows::Kept
        || left_key.is_empty()
        || left_key.len() != right_key.len()
    {
        return None;
    }
    verified_index(index, right, right_key, indexes)?;
    let positions = operands.map(usize::from);
    let left = &stages.get(positions[0])?.fields;
    let right = &stages.get(positions[1])?.fields;
    let l = left_key
        .iter()
        .copied()
        .map(usize::from)
        .collect::<Vec<_>>();
    let r = right_key
        .iter()
        .copied()
        .map(usize::from)
        .collect::<Vec<_>>();
    let links = l
        .iter()
        .zip(&r)
        .map(|(&a, &b)| Some((*left.get(a)?, *right.get(b)?)))
        .collect::<Option<Vec<_>>>()?;
    let symbol_field = |field| matches!(field, Field::Kind | Field::Text | Field::Role);
    if !links
        .iter()
        .all(|&(EntityColumn(a, _), EntityColumn(b, _))| {
            a == b || (symbol_field(a) && symbol_field(b))
        })
    {
        return None;
    }
    let (expr, fields) = match (probe.mode, probe.existence) {
        (ProbeMode::Anti, Existence::First) if probe.empty_right == EmptyRight::EveryLeftRow => (
            Operation::Anti {
                l: left_key.clone(),
                r: right_key.clone(),
            },
            left.clone(),
        ),
        (ProbeMode::Inner, Existence::Every) if !probe.output.is_empty() => {
            let drive_left = links.iter().find_map(|&(EntityColumn(a, depth), b)| {
                (a == Field::Id && b == EntityColumn(Field::Id, AncestorDistance(0)))
                    .then_some((Side::Left, depth))
            });
            let (side, depth) = drive_left.or_else(|| {
                links.iter().find_map(|&(a, EntityColumn(b, depth))| {
                    (a == EntityColumn(Field::Id, AncestorDistance(0)) && b == Field::Id)
                        .then_some((Side::Right, depth))
                })
            })?;
            let fields: Vec<EntityColumn> = probe
                .output
                .iter()
                .map(|c| {
                    let (layout, shift) = match c.side {
                        Side::Left => (left, if side == Side::Right { depth.0 } else { 0 }),
                        Side::Right => (right, if side == Side::Left { depth.0 } else { 0 }),
                    };
                    let &EntityColumn(field, at) = layout.get(usize::from(c.column))?;
                    Some(EntityColumn(
                        field,
                        AncestorDistance(if field == Field::File {
                            0
                        } else {
                            at.0.checked_add(shift)?
                        }),
                    ))
                })
                .collect::<Option<_>>()?;
            (
                Operation::Join {
                    side,
                    depth,
                    l: left_key.clone(),
                    r: right_key.clone(),
                },
                fields,
            )
        }
        _ => return None,
    };
    Some(Stage {
        expr,
        fields,
        operands: operands.to_vec(),
        physical: Some(EntityExportId(physical.0)),
    })
}

fn verified_index(
    index: ProbeIndex,
    source: RowSource,
    columns: &[ColumnIndex],
    indexes: &[KeyedIndex],
) -> Option<()> {
    match index {
        ProbeIndex::Own => Some(()),
        ProbeIndex::Shared(at) => indexes
            .get(at.0)
            .filter(|shared| {
                shared.source == source
                    && matches!(&shared.key, IndexKey::Cells(key) if key == columns)
            })
            .map(|_| ()),
        _ => None,
    }
}
