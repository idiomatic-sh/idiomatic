//! The `layout` service: free functions named by verb that lay a plan out for the worker that runs it.

use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::service::kinds as relation_kinds;
use ryo_language::domain::relations::types::{CellKind, FileId, Relation};

use crate::domain::lowering::service::lower;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::service::compute;
use crate::domain::operator::types::{KeptSteps, PlanIdentity, PlanLayout, ReadCount, StepPlace};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::{Frontier, Rows, Table};
use crate::domain::trust::service::needs_of;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Plan, StepScope};

/// How many steps read the step `id`, the plan's emitting and handing over counting as reads.
#[must_use]
pub(crate) fn reads_of(plan: &Plan, id: StepPlace) -> ReadCount {
    let id = id.0;
    let steps = plan
        .steps
        .iter()
        .filter(|step| step.scope == StepScope::PerFile)
        .map(|step| {
            named(&inputs_of(&step.operation))
                .iter()
                .filter(|input| {
                    **input == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX)))
                })
                .count()
        })
        .sum::<usize>();
    let handed = plan
        .frontier
        .iter()
        .filter(|input| **input == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX))))
        .count();
    ReadCount(steps + handed + usize::from(usize::try_from(plan.emit.step.0) == Ok(id)))
}

/// The layout of a plan: which steps keep their rows, and what columns each answers.
///
/// # Errors
///
/// `NotRun` when a step reads a step that has no columns, `NotRead` when `read` has no rows for a relation, and whatever an
/// operator refuses.
pub(crate) fn layout_of<'f>(
    plan: &Plan,
    index: PlanIdentity,
    read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    context: &mut OperatorContext<'_>,
) -> Result<PlanLayout, OperatingError> {
    let count = plan.steps.len();
    let emitted = usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX);
    // A step keeps its rows when it is the one emitted, is handed to the cross-file steps, or is read by more than one step.
    let kept: Vec<bool> =
        plan.steps
            .iter()
            .enumerate()
            .map(|(id, step)| {
                let handed = plan.frontier.iter().any(|input| {
                    *input == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX)))
                });
                step.scope != StepScope::CrossFile
                    && (id == emitted || handed || reads_of(plan, StepPlace(id)).0 > 1)
            })
            .collect();
    // What each step answers, found by running each on inputs with the columns and no rows: the tree has no element.
    let nothing = Elements::default();
    let mut kinds: Vec<Vec<CellKind>> = vec![Vec::new(); count];
    let mut scratch_out = Table::default();
    for (id, step) in plan.steps.iter().enumerate() {
        if step.scope == StepScope::CrossFile {
            continue;
        }
        let inputs = inputs_of(&step.operation);
        let mut empties: [Table; 3] = [Table::default(), Table::default(), Table::default()];
        for (empty, input) in empties.iter_mut().zip(named(&inputs)) {
            if let Input::Step(source) = input {
                empty.kinds = kinds
                    .get(usize::try_from(source.0).unwrap_or(usize::MAX))
                    .cloned()
                    .ok_or(OperatingError::NotRun)?;
            }
        }
        let mut rows: Vec<Rows<'_>> = Vec::new();
        for (empty, input) in empties.iter().zip(named(&inputs)) {
            rows.push(match input {
                Input::Step(_) => Rows::Computed(empty),
                Input::Relation(relation @ (Relation::SyntaxTree | Relation::SyntaxChild)) => {
                    // The tree must be readable, though what is run here reads none of its elements.
                    read(*relation).map(|_| ())?;
                    Rows::Tree(TreeRows {
                        elements: &nothing,
                        file: FileId(0),
                        form: if *relation == Relation::SyntaxChild {
                            TreeForm::Edges
                        } else {
                            TreeForm::Elements
                        },
                    })
                }
                Input::Relation(relation) => read(*relation)?,
            });
        }
        compute(&step.operation, &rows, context, &mut scratch_out)?;
        if let Some(answered) = kinds.get_mut(id) {
            answered.clone_from(&scratch_out.kinds);
        }
    }
    Ok(PlanLayout {
        plan: index,
        kept: KeptSteps(kept),
        kinds,
        lowering: lower(plan),
        needs: needs_of(plan),
    })
}

/// The inputs handed to the steps after the files, each with the columns it has. A table the workers gave no row of has no kinds, for
/// the words of a table say them with its rows (a rule that governs no file, a relation no file wrote): the columns the input has are
/// then those the plan says, so that the steps after it find the columns they ask for.
///
/// # Errors
///
/// Whatever an operator refuses when the columns of the steps are worked out.
pub(crate) fn shaped(
    plan: &Plan,
    frontier: &[Frontier],
    context: &mut OperatorContext<'_>,
) -> Result<Vec<Frontier>, OperatingError> {
    if frontier.iter().all(|each| !each.table.kinds.is_empty()) {
        return Ok(frontier.to_vec());
    }
    let tables: Vec<(Relation, Table)> = plan
        .steps
        .iter()
        .flat_map(|step| named(&inputs_of(&step.operation)).to_vec())
        .filter_map(|input| match input {
            Input::Relation(relation) => Some(relation),
            Input::Step(_) => None,
        })
        .map(|relation| {
            (
                relation,
                Table {
                    kinds: relation_kinds(relation).to_vec(),
                    ..Table::default()
                },
            )
        })
        .collect();
    let read = |relation: Relation| -> Result<Rows<'_>, OperatingError> {
        tables
            .iter()
            .find(|(each, _)| *each == relation)
            .map(|(_, table)| Rows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let layout = layout_of(plan, PlanIdentity(0), &read, context)?;
    frontier
        .iter()
        .map(|each| {
            let mut shaped = each.clone();
            if shaped.table.kinds.is_empty() {
                shaped.table.kinds = match each.input {
                    Input::Relation(relation) => relation_kinds(relation).to_vec(),
                    Input::Step(id) => layout
                        .kinds
                        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                        .cloned()
                        .ok_or(OperatingError::NotRun)?,
                };
            }
            Ok(shaped)
        })
        .collect()
}
