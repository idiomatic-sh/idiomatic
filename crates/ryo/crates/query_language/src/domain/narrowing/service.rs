//! The `narrowing` service: free functions named by verb over the per-file inputs of joins after the files, narrowed in each file to
//! the rows whose key the other input's values can meet.

use super::types::Narrowed;
use crate::domain::inlining::service::columns_read;
use crate::domain::inlining::service::projection_column;
use crate::domain::inlining::service::reading_through;
use crate::domain::lineage::service::column_kind;
use crate::domain::lineage::service::value_origins;
use crate::domain::lineage::types::ValueOrigin;
use crate::domain::lowering::types::FileOrigin;
use crate::domain::planning::service::planned_scope;
use crate::domain::planning::service::stated_by;
use crate::domain::rewriting::types::Planned;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The plan with the per-file input `R` of each join after the files `Join(L, R)` on a key that says the file narrowed to `R ⋉ O` in
/// each file, where `O` are the per-file steps the values of `L`'s key come from: a row of `R` no value of `O` meets in its own file
/// meets no row of `L`, so a file hands over only the rows that can. A key whose values the plan cannot follow to per-file steps
/// leaves the join as it is. A node key is chosen before a file key.
#[must_use]
pub(crate) fn narrow_lookups(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = narrow_lookups_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn narrow_lookups_roots(
    mut steps: Vec<PlanStep>,
    roots: &mut [StepId],
) -> Vec<PlanStep> {
    let mut place = 0;
    while place < steps.len() {
        let Some(Narrowed { local, kept, made }) = narrowed(&steps, place) else {
            place += 1;
            continue;
        };
        let width = made.len();
        let count = u32::try_from(width).unwrap_or(u32::MAX);
        let first = u32::try_from(place).unwrap_or(u32::MAX);
        for step in &mut steps {
            each_input(&mut step.operation, &mut |input| {
                if let Input::Step(id) = input
                    && id.0 >= first
                {
                    id.0 += count;
                }
            });
        }
        for root in roots.iter_mut() {
            if root.0 >= first {
                root.0 += count;
            }
        }
        for (offset, step) in made.into_iter().enumerate() {
            steps.insert(place + offset, step);
        }
        let through = Input::Step(StepId(first + count - 1));
        if let Some(join) = steps.get_mut(place + width) {
            reading_through(&mut join.operation, local, through, &|column| {
                projection_column(&kept, column)
            });
        }
        place += width + 1;
    }
    steps
}

/// Whether the values of every origin say which file they are of.
fn origins_state_file(steps: &[PlanStep], origins: &[ValueOrigin]) -> bool {
    origins.iter().all(|origin| {
        column_kind(steps, origin.input, origin.column)
            .is_some_and(|kind| stated_by(kind) == FileOrigin::Stated)
    })
}

/// The per-file input of the join at `place` and the steps that narrow it: one projection of the values of each origin, their union,
/// and the join of the input with it.
fn narrowed(steps: &[PlanStep], place: usize) -> Option<Narrowed> {
    let step = steps.get(place)?;
    if step.scope != StepScope::CrossFile {
        return None;
    }
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        filter,
        view,
        reach: None,
        ..
    } = &step.operation
    else {
        return None;
    };
    if !filter.is_empty() || !view.is_empty() {
        return None;
    }
    let (crossing, local, crossing_keys, local_keys) =
        match (planned_scope(steps, *left)?, planned_scope(steps, *right)?) {
            (StepScope::CrossFile, StepScope::PerFile) => (*left, *right, left_on, right_on),
            (StepScope::PerFile, StepScope::CrossFile) => (*right, *left, right_on, left_on),
            (StepScope::PerFile, StepScope::PerFile)
            | (StepScope::CrossFile, StepScope::CrossFile) => return None,
        };
    let kept = columns_read(&step.operation, local)?;
    // A node key first, then a file key: a node meets fewer rows.
    let (local_key, origins) = [CellKind::Node, CellKind::File].iter().find_map(|wanted| {
        local_keys
            .iter()
            .zip(crossing_keys)
            .find_map(|(local_key, crossing_key)| {
                if column_kind(steps, local, *local_key) != Some(*wanted) {
                    return None;
                }
                let origins = value_origins(steps, crossing, *crossing_key)?;
                let narrows = origins_state_file(steps, &origins)
                    && !origins.contains(&ValueOrigin {
                        input: local,
                        column: *local_key,
                    });
                narrows.then_some((*local_key, origins))
            })
    })?;
    let first = u32::try_from(place).ok()?;
    let at = |offset: usize| -> Option<Input> {
        Some(Input::Step(StepId(first + u32::try_from(offset).ok()?)))
    };
    let mut made: Vec<PlanStep> = origins
        .iter()
        .map(|origin| PlanStep {
            operation: Operation::Project {
                from: origin.input,
                columns: vec![origin.column],
                distinct: RepeatedRows::Dropped,
            },
            scope: StepScope::PerFile,
        })
        .collect();
    let mut values = at(0)?;
    for offset in 1..origins.len() {
        let union = at(made.len())?;
        made.push(PlanStep {
            operation: Operation::Union {
                left: values,
                right: at(offset)?,
            },
            scope: StepScope::PerFile,
        });
        values = union;
    }
    made.push(PlanStep {
        operation: Operation::Join {
            left: local,
            right: values,
            left_on: vec![local_key],
            right_on: vec![ColumnIndex(0)],
            output: kept
                .iter()
                .map(|column| JoinColumn {
                    side: Side::Left,
                    column: *column,
                })
                .collect(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        scope: StepScope::PerFile,
    });
    Some(Narrowed { local, kept, made })
}
