//! Recognize only the reviewed Join/Select/Project right subtree, never rewrite the antijoin.
use super::types::*;
use crate::domain::operator::types::PlanIdentity;
use ryo_relations::domain::relations::{service::kinds, types::CellKind};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, Operand, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::{
    service::{inputs_of, named},
    types::{Operation, Plan, StepScope},
};

fn step(plan: &Plan, id: StepId) -> Result<&Operation, Decline> {
    let step = plan
        .steps
        .get(id.0 as usize)
        .ok_or(Decline::InvalidColumn)?;
    if step.scope != StepScope::PerFile {
        return Err(Decline::CrossFile);
    }
    Ok(&step.operation)
}
fn use_counts(plan: &Plan) -> Vec<usize> {
    let mut counts = vec![0usize; plan.steps.len()];
    for step in &plan.steps {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(id) = input {
                if let Some(count) = counts.get_mut(id.0 as usize) {
                    *count += 1;
                }
            }
        }
    }
    for input in plan
        .frontier
        .iter()
        .copied()
        .chain(std::iter::once(Input::Step(plan.emit.step)))
    {
        if let Input::Step(id) = input {
            if let Some(count) = counts.get_mut(id.0 as usize) {
                *count += 1;
            }
        }
    }
    counts
}
fn from_step(input: Input) -> Result<StepId, Decline> {
    match input {
        Input::Step(id) => Ok(id),
        _ => Err(Decline::NotPlainSelfJoin),
    }
}

/// Decode lineage only through value-preserving operations. No metadata-only proof of
/// arbitrary hosted/Rows values is accepted. Stored Number decoding itself is total.
fn number_origins(
    plan: &Plan,
    input: Input,
    column: ColumnIndex,
    before: usize,
) -> Option<Vec<StoredNumberOrigin>> {
    match input {
        // A named publication needs an explicit admitted-number lineage proof;
        // relation metadata is not authority for arbitrary producer values.
        Input::BoundObservation(_) => None,
        Input::Relation(relation) => (kinds(relation).get(column.0 as usize)
            == Some(&CellKind::Number))
        .then_some(vec![StoredNumberOrigin { relation, column }]),
        Input::Step(id) => {
            let at = id.0 as usize;
            if at >= before {
                return None;
            }
            let operation = &plan.steps.get(at)?.operation;
            let (from, column) = match operation {
                Operation::Select { from, kept, .. } => (
                    *from,
                    if kept.is_empty() {
                        column
                    } else {
                        *kept.get(column.0 as usize)?
                    },
                ),
                Operation::Project { from, columns, .. } => {
                    (*from, *columns.get(column.0 as usize)?)
                }
                Operation::Join {
                    left,
                    right,
                    output,
                    view,
                    reach,
                    ..
                } => {
                    if !view.is_empty() || reach.is_some() {
                        return None;
                    }
                    let mapped = output.get(column.0 as usize)?;
                    (
                        if mapped.side == Side::Left {
                            *left
                        } else {
                            *right
                        },
                        mapped.column,
                    )
                }
                Operation::Antijoin { left, .. }
                | Operation::Difference { left, .. }
                | Operation::Intersection { left, .. } => (*left, column),
                Operation::Union { left, right } => {
                    let mut origins = number_origins(plan, *left, column, at)?;
                    for origin in number_origins(plan, *right, column, at)? {
                        if !origins.contains(&origin) {
                            origins.push(origin);
                        }
                    }
                    return Some(origins);
                }
                _ => return None,
            };
            number_origins(plan, from, column, at)
        }
    }
}

/// This API consumes the ORIGINAL plan and its exact layout. A canonical DAG operation
/// with placeholder Step(0) edges is not an input to this recognizer. Runtime binding
/// must additionally match the descriptor against resolved DAG producer identities.
fn recognize<'a>(
    plan: &Plan,
    schemas: &dyn Fn(StepId) -> Option<&'a [CellKind]>,
    readers: &[usize],
    anti: StepId,
) -> Result<Shape, Decline> {
    let Operation::Antijoin { right, .. } = step(plan, anti)? else {
        return Err(Decline::NotAntijoin);
    };
    let project = from_step(*right)?;
    let Operation::Project {
        from,
        columns,
        distinct: RepeatedRows::Dropped,
    } = step(plan, project)?
    else {
        return Err(Decline::NotDistinctProject);
    };
    let select = from_step(*from)?;
    let Operation::Select {
        from,
        comparisons,
        kept,
    } = step(plan, select)?
    else {
        return Err(Decline::NotSinglePredicate);
    };
    let [comparison] = comparisons.as_slice() else {
        return Err(Decline::NotSinglePredicate);
    };
    if !kept.is_empty() {
        return Err(Decline::NotSinglePredicate);
    }
    let join = from_step(*from)?;
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
        reach: None,
    } = step(plan, join)?
    else {
        return Err(Decline::NotPlainSelfJoin);
    };
    if !filter.is_empty() || !view.is_empty() {
        return Err(Decline::NotPlainSelfJoin);
    }
    if left != right {
        return Err(Decline::DifferentSource);
    }
    let source = from_step(*left)?;
    if left_on != right_on {
        return Err(Decline::DifferentPartitionColumns);
    }
    if [join, select, project]
        .iter()
        .any(|id| readers.get(id.0 as usize).copied() != Some(1))
    {
        return Err(Decline::ExtraConsumer);
    }
    let _ = step(plan, source)?;
    let (Operand::Column(a), Operand::Column(b)) = (&comparison.left, &comparison.right) else {
        return Err(Decline::NonStrictOrder);
    };
    let a = output.get(a.0 as usize).ok_or(Decline::InvalidColumn)?;
    let b = output.get(b.0 as usize).ok_or(Decline::InvalidColumn)?;
    if a.column != b.column || a.side == b.side {
        return Err(Decline::DifferentOrderColumns);
    }
    let extremum = match (comparison.test, a.side) {
        (Test::LessThan, Side::Left) | (Test::GreaterThan, Side::Right) => Extremum::Maximum,
        (Test::GreaterThan, Side::Left) | (Test::LessThan, Side::Right) => Extremum::Minimum,
        _ => return Err(Decline::NonStrictOrder),
    };
    let schema = schemas(source).ok_or(Decline::InvalidColumn)?;
    if schema.get(a.column.0 as usize) != Some(&CellKind::Number) {
        return Err(Decline::NotNumber);
    }
    if output
        .iter()
        .any(|column| column.column.0 as usize >= schema.len())
    {
        return Err(Decline::InvalidColumn);
    }
    if left_on
        .iter()
        .any(|column| column.0 as usize >= schema.len())
    {
        return Err(Decline::InvalidColumn);
    }
    let mut projected_left = Vec::with_capacity(columns.len());
    for column in columns {
        let projected = output
            .get(column.0 as usize)
            .ok_or(Decline::InvalidColumn)?;
        if projected.side != Side::Left {
            return Err(Decline::RightProjection);
        }
        if projected.column.0 as usize >= schema.len() {
            return Err(Decline::InvalidColumn);
        }
        projected_left.push(projected.column);
    }
    let origins = number_origins(plan, Input::Step(source), a.column, join.0 as usize)
        .ok_or(Decline::UnprovenNumberOrigin)?;
    Ok(Shape {
        plan: PlanIdentity(std::ptr::from_ref(plan).addr()),
        source,
        replaced_join: join,
        replaced_select: select,
        retained_project: project,
        retained_antijoin: anti,
        partition: left_on.clone(),
        order: a.column,
        extremum,
        projected_left,
        number: NumberRequirement {
            source,
            column: a.column,
            width: schema.len(),
            origins,
        },
    })
}

pub(crate) fn compile<'a>(
    plan: &Plan,
    schemas: &dyn Fn(StepId) -> Option<&'a [CellKind]>,
) -> Program {
    let readers = use_counts(plan);
    let mut program = Program::default();
    for (at, entry) in plan.steps.iter().enumerate() {
        if !matches!(entry.operation, Operation::Antijoin { .. }) {
            continue;
        }
        let Ok(at) = u32::try_from(at) else {
            continue;
        };
        match recognize(plan, schemas, &readers, StepId(at)) {
            Ok(shape) => program.shapes.push(shape),
            Err(reason) => program.declines.push((StepId(at), reason)),
        }
    }
    program
}

#[cfg(test)]
#[path = "service_test.rs"]
mod tests;
