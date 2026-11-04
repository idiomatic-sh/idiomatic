//! Rule output orchestration over the mandatory graph scheduler.
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    operator::types::{Pool, PreparedPool},
    table_cells::types::TreeRows,
    table_model::{service::materialize, types::Rows},
    trust::{service::blocked, types::Blocking},
};
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::{
    plan_parts::types::Input,
    plans::types::{Plan, StepScope},
    rows::types::RowCount,
};
#[cfg(test)]
pub(crate) fn evaluate_file<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],
) -> Result<(), OperatingError> {
    evaluate_file_with_dag(
        plan, relations, tree, context, pool, prepared, classes, None, 0, None,
    )
}

#[cfg(test)]
pub(crate) fn evaluate_file_with_graph<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],

    rule: usize,
) -> Result<(), OperatingError> {
    evaluate_file_with_fixture::<true>(
        plan, relations, tree, context, pool, prepared, classes, rule, None,
    )
}

#[cfg(test)]
pub(crate) fn evaluate_file_with_dag<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],

    rule: usize,
    file_dag: Option<&dyn crate::application::file_dag::interface::DagExecution>,
) -> Result<(), OperatingError> {
    evaluate_file_with_fixture::<false>(
        plan, relations, tree, context, pool, prepared, classes, rule, file_dag,
    )
}

/// The old retained-cut schedule is an explicit test oracle. Ordinary fixture
/// calls instantiate the same no-legacy evaluator as production.
#[cfg(test)]
fn evaluate_file_with_fixture<'f, const LEGACY: bool>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],

    rule: usize,
    file_dag: Option<&dyn crate::application::file_dag::interface::DagExecution>,
) -> Result<(), OperatingError> {
    let request = crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
        std::slice::from_ref(plan),
        &[context.bindings.to_vec()],
        &[],
        &|relation| {
            relations(relation).map_or_else(
                || ryo_relations::domain::relations::service::kinds(relation),
                |view| view.kinds,
            )
        },
        crate::outbound::derived::factory::test_port(),
    )?;
    evaluate_file_inner::<LEGACY>(
        plan,
        relations,
        tree,
        context,
        pool,
        prepared,
        classes,
        rule,
        file_dag,
        &crate::application::file_dag::execution::CommonOperatorEngine::fixture(),
        &request.rules[0],
    )
}

pub(crate) fn evaluate_file_with_executor<'f>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],

    rule: usize,
    file_dag: &dyn crate::application::file_dag::interface::DagExecution,
    executor: &crate::application::file_dag::executor::ScopedExecutor<'_>,
    operators: &[std::sync::Arc<crate::domain::operator::contract::types::OperatorContract>],
) -> Result<(), OperatingError> {
    evaluate_file_inner::<false>(
        plan, relations, tree, context, pool, prepared, classes, rule, file_dag, executor,
        operators,
    )
}

fn evaluate_file_inner<'f, const LEGACY: bool>(
    plan: &Plan,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    tree: Option<TreeRows<'f>>,
    context: &mut OperatorContext<'_>,
    pool: &mut Pool,
    prepared: &mut PreparedPool,
    classes: &[crate::domain::positions::types::Slots],

    rule: usize,
    file_dag: &dyn crate::application::file_dag::interface::DagExecution,
    executor: &crate::application::file_dag::executor::ScopedExecutor<'_>,
    operators: &[std::sync::Arc<crate::domain::operator::contract::types::OperatorContract>],
) -> Result<(), OperatingError> {
    // Rule orchestration declares roots only. Every dependency is demanded by
    // the required graph owner, including non-retainable and incomplete inputs.
    let dag = file_dag;
    if !std::rc::Rc::ptr_eq(executor.owner(), &dag.relation_owner()) {
        return Err(OperatingError::NotRead);
    }
    dag.begin_rule(rule)?;
    let read = |relation: Relation| -> Result<Rows<'f>, OperatingError> {
        match (
            crate::domain::table_cells::service::native_form(relation),
            tree,
        ) {
            (Some(form), Some(tree)) => Ok(Rows::Tree(TreeRows { form, ..tree })),
            _ => relations(relation)
                .map(Rows::Stored)
                .ok_or(OperatingError::NotRead),
        }
    };
    let owner = executor.owner();
    let mut emitted = None;
    let mut emit_error = None;
    // Named roots are independent of Emit: retain each refusal without erasing
    // another root's completed publication.
    for (index, output) in plan.observations.iter().enumerate() {
        if output.scope != StepScope::PerFile {
            continue;
        }
        let Input::Step(step) = output.source else {
            return Err(OperatingError::NotRun);
        };
        let result = dag
            .root(rule, step.0 as usize)
            .and_then(|node| dag.demand(node, &read, executor, context));
        match result {
            Ok(publication) => {
                if step == plan.emit.step {
                    emitted = Some(publication);
                }
            }
            Err(error) => {
                dag.refuse_observation(
                    rule,
                    ryo_values::domain::observations::types::ObservationIndex(
                        u32::try_from(index).map_err(|_| OperatingError::NotRun)?,
                    ),
                    ryo_values::domain::observations::types::ObservationReason(
                        error.to_string().into(),
                    ),
                )?;
                if step == plan.emit.step {
                    emit_error = Some(error);
                }
            }
        }
    }
    for (input, into) in plan.frontier.iter().zip(pool.frontier.iter_mut()) {
        match input {
            Input::Relation(relation) => {
                materialize(&read(*relation)?, into, &mut context.scratch.row)
            }
            Input::Step(step) => {
                let publication =
                    dag.demand(dag.root(rule, step.0 as usize)?, &read, executor, context)?;
                materialize(
                    &owner.resolve(publication.as_ref(), &read)?,
                    into,
                    &mut context.scratch.row,
                );
            }
            Input::BoundObservation(id) => {
                if plan
                    .observation_inputs
                    .get(id.0 as usize)
                    .is_some_and(|schema| schema.scope == StepScope::CrossFile)
                {
                    continue;
                }
                let publication = dag.observation(
                    crate::domain::file_program::types::ObservationSlot { rule, input: *id },
                    &read,
                    executor,
                    context,
                )?;
                materialize(
                    &owner.resolve(publication.as_ref(), &read)?,
                    into,
                    &mut context.scratch.row,
                );
            }
        }
    }
    if let Some(error) = emit_error {
        return Err(error);
    }
    let emit = plan.emit.step.0 as usize;
    if plan
        .steps
        .get(emit)
        .is_some_and(|step| step.scope == StepScope::PerFile)
    {
        let target = pool.steps.get_mut(emit).ok_or(OperatingError::NotRun)?;
        let publication = match emitted {
            Some(publication) => publication,
            None => dag.demand(dag.root(rule, emit)?, &read, executor, context)?,
        };
        materialize(
            &owner.resolve(publication.as_ref(), &read)?,
            target,
            &mut context.scratch.row,
        );
    }
    if let Some(target) = prepared.fed.get_mut(rule) {
        for (total, more) in target.iter_mut().zip(dag.counts(rule)?) {
            crate::domain::workers::service::add_fed(total, &more);
        }
    }
    Ok(())
}

use ryo_values::domain::rows::types::Table;
