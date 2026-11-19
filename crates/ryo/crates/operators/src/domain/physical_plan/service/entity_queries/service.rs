//! Compile after the physical rewrites, including the sources hidden behind lookup indexes.

use super::{filtering, joining, projection};
use crate::domain::entity_queries::types::{
    AncestorDistance, EntityAttribute as Field, EntityColumn, EntityExport, EntityExportId,
    EntityQueryOp as Operation, EntityStage as Stage, entity_plan::value::QueryPlan,
};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{KeyedIndex, PlanNodeId, ProbeIndex, RowSource};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{RepeatedRows, StepId};

/// Compile every supported file-local stage that reaches an engine use. Export the
/// stages an ordinary consumer or the engine can observe by their original node IDs.
pub(crate) fn compile(nodes: &[PhysicalNode], indexes: &[KeyedIndex]) -> QueryPlan {
    let mut live = vec![false; nodes.len()];
    for (place, node) in nodes.iter().enumerate().rev() {
        if node.engine.0 == 0 && !live[place] {
            continue;
        }
        live[place] = true;
        for source in reads(&node.lowering, indexes) {
            if let RowSource::Node(parent) = source {
                let Ok(parent) = usize::try_from(parent.0) else {
                    return QueryPlan::default();
                };
                if parent >= place {
                    return QueryPlan::default();
                }
                live[parent] = true;
            }
        }
    }
    let mut stages = vec![
        Stage {
            expr: Operation::Syntax,
            fields: vec![
                EntityColumn(Field::Id, AncestorDistance(0)),
                EntityColumn(Field::Kind, AncestorDistance(0)),
                EntityColumn(Field::Text, AncestorDistance(0)),
                EntityColumn(Field::File, AncestorDistance(0)),
                EntityColumn(Field::Span, AncestorDistance(0)),
            ],
            operands: Vec::new(),
            physical: None,
        },
        Stage {
            expr: Operation::Edge,
            fields: vec![
                EntityColumn(Field::Id, AncestorDistance(1)),
                EntityColumn(Field::Id, AncestorDistance(0)),
                EntityColumn(Field::Ordinal, AncestorDistance(0)),
                EntityColumn(Field::Role, AncestorDistance(0)),
                EntityColumn(Field::File, AncestorDistance(0)),
            ],
            operands: Vec::new(),
            physical: None,
        },
    ];
    let mut mapped = vec![None; nodes.len()];
    let mut exposed = vec![false; nodes.len()];
    for (place, node) in nodes.iter().enumerate().filter(|(place, _)| live[*place]) {
        let Ok(id) = u32::try_from(place) else {
            return QueryPlan::default();
        };
        if let Some(stage) = lower(PlanNodeId(id), &node.lowering, indexes, &mapped, &stages) {
            let Ok(step) = u32::try_from(stages.len()) else {
                return QueryPlan::default();
            };
            mapped[place] = Some(StepId(step));
            exposed[place] = node.engine.0 > 0;
            stages.push(stage);
        } else {
            for source in reads(&node.lowering, indexes) {
                if let RowSource::Node(parent) = source
                    && let Ok(parent) = usize::try_from(parent.0)
                    && mapped.get(parent).copied().flatten().is_some()
                {
                    exposed[parent] = true;
                }
            }
        }
    }
    let mut exports = Vec::new();
    for ((visible, stage), physical) in exposed.into_iter().zip(mapped).zip(0_u32..) {
        if visible && let Some(step) = stage {
            exports.push(EntityExport(EntityExportId(physical), step));
        }
    }
    if exports.is_empty() {
        QueryPlan::default()
    } else {
        QueryPlan::from_stages(stages, exports)
    }
}

/// A lookup omits its source in Lowering::sources because the shared index owns that read.
fn reads<'a>(
    lowering: &'a Lowering,
    indexes: &'a [KeyedIndex],
) -> impl Iterator<Item = RowSource> + 'a {
    let from_index = match lowering {
        Lowering::Lookup { index, .. } => indexes.get(index.0).map(|shared| shared.source),
        Lowering::Probe {
            right,
            index: ProbeIndex::Shared(index),
            ..
        } => indexes
            .get(index.0)
            .map(|shared| shared.source)
            .filter(|source| source != right),
        _ => None,
    };
    lowering.sources().chain(from_index)
}

fn operand(source: RowSource, mapped: &[Option<StepId>]) -> Option<StepId> {
    match source {
        RowSource::Facts(Relation::SemanticSyntax) => Some(StepId(0)),
        RowSource::Facts(Relation::SemanticChild) => Some(StepId(1)),
        RowSource::Node(node) => mapped
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            .copied()
            .flatten(),
        _ => None,
    }
}

fn lower(
    node: PlanNodeId,
    lowering: &Lowering,
    indexes: &[KeyedIndex],
    mapped: &[Option<StepId>],
    stages: &[Stage],
) -> Option<Stage> {
    match lowering {
        Lowering::Project { source, project } => {
            if project.columns.is_empty() {
                return None;
            }
            let input = operand(*source, mapped)?;
            Some(Stage {
                expr: Operation::Project {
                    distinct: project.distinct,
                },
                fields: projection::service::columns(
                    &stages.get(usize::from(input))?.fields,
                    &project.columns,
                )?,
                operands: vec![input],
                physical: Some(EntityExportId(node.0)),
            })
        }
        Lowering::Lookup { index, key, select } => {
            let shared = indexes.get(index.0)?;
            let input = operand(filtering::service::lookup(shared, *key, select)?, mapped)?;
            filtering::service::filter(&stages.get(usize::from(input))?.fields, select, input, node)
        }
        Lowering::Filter { source, select } => {
            let input = operand(*source, mapped)?;
            filtering::service::filter(&stages.get(usize::from(input))?.fields, select, input, node)
        }
        Lowering::Distinct {
            sources: [left, right],
        } => {
            let left = operand(*left, mapped)?;
            let right = operand(*right, mapped)?;
            let fields = &stages.get(usize::from(left))?.fields;
            (fields == &stages.get(usize::from(right))?.fields).then(|| Stage {
                expr: Operation::Union,
                fields: fields.clone(),
                operands: vec![left, right],
                physical: Some(EntityExportId(node.0)),
            })
        }
        Lowering::Probe {
            left,
            right,
            probe,
            index,
        } => {
            let left = operand(*left, mapped)?;
            let right_stage = operand(*right, mapped)?;
            joining::service::probe(
                probe,
                *index,
                *right,
                indexes,
                [left, right_stage],
                stages,
                node,
            )
        }
        _ => None,
    }
}
