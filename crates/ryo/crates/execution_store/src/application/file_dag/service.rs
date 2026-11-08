//! Prepare every operation once for one request, shared by all file workers.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operation_schema::types::InputShape;
use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
use ryo_execution_domain::domain::operator::service::prepare_operator;
use ryo_execution_domain::domain::table_cells::types::TreeForm;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Relation},
};
use ryo_values::domain::hosted::types::{HostKind, HostRelation};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::{
    service::{inputs_of, named},
    types::{Operation, Plan},
};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

use ryo_execution_domain::domain::operator::contract::types::RequestOperators;

pub fn prepare(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[HostRelation],
    semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
) -> Result<RequestOperators, OperatingError> {
    prepare_with_schemas(plans, bindings, hosted, &kinds, semantic)
}
pub fn prepare_with_schemas<'s>(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[HostRelation],
    relation_kinds: &dyn Fn(Relation) -> &'s [CellKind],
    semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
) -> Result<RequestOperators, OperatingError> {
    let semantic: crate::application::file_dag::derived::owner::types::DerivedAccess = semantic.into();
    let host_kinds = hosted
        .iter()
        .map(|relation| {
            relation
                .columns
                .iter()
                .map(|column| match column.kind {
                    HostKind::File => CellKind::File,
                    HostKind::Text => CellKind::Symbol,
                    HostKind::Number => CellKind::Number,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut views = crate::application::file_dag::derived::request::RequestViews::new(semantic.clone());
    let mut rules = Vec::with_capacity(plans.len());
    let mut preparations = 0;
    for (rule, plan) in plans.iter().enumerate() {
        let observed_kinds: Vec<Vec<CellKind>> = plan
            .observation_inputs
            .iter()
            .map(|input| input.columns.iter().map(|column| column.kind).collect())
            .collect();
        let mut contracts: Vec<Arc<OperatorContract>> = Vec::with_capacity(plan.steps.len());
        let mut tree_outputs: Vec<Option<TreeForm>> = Vec::with_capacity(plan.steps.len());
        for step in &plan.steps {
            let dependencies = inputs_of(&step.operation);
            let mut schemas = named(&dependencies)
                .iter()
                .map(|input| {
                    Ok(match input {
                        Input::Relation(relation) => InputShape {
                            kinds: relation_kinds(*relation),
                            tree: ryo_execution_domain::domain::table_cells::service::native_form(*relation),
                        },
                        Input::BoundObservation(id) => InputShape {
                            kinds: observed_kinds
                                .get(
                                    usize::try_from(id.0)
                                        .map_err(|_| OperatingError::NotRun)?,
                                )
                                .ok_or(OperatingError::NotRun)?,
                            tree: None,
                        },
                        Input::Step(id) => InputShape {
                            kinds: contracts
                                .get(id.0 as usize)
                                .map_or(&[], |contract| match &contract.schema {
                                    Ok(kinds) => kinds.as_slice(),
                                    Err(_) => &[],
                                }),
                            tree: tree_outputs.get(id.0 as usize).copied().flatten(),
                        },
                    })
                })
                .collect::<Result<Vec<_>, OperatingError>>()?;
            // Hosted is a real request source. Its schema is not a pretend
            // dependency and no evaluator special-case may bypass registration.
            if let Operation::Hosted { relation } = &step.operation {
                schemas.push(InputShape {
                    kinds: host_kinds
                        .get(relation.0 as usize)
                        .map_or(&[], Vec::as_slice),
                    tree: None,
                });
            }
            let mut contract = prepare_operator(
                kind_of(&step.operation),
                &step.operation,
                &schemas,
                bindings.get(rule).map_or(&[], Vec::as_slice),
            )?;
            views.bind(&mut contract)?;
            let tree = match &step.operation {
                Operation::Select {
                    comparisons, kept, ..
                } if comparisons.is_empty() && kept.is_empty() => {
                    schemas.first().and_then(|input| input.tree)
                }
                Operation::Project {
                    columns, distinct, ..
                } if *distinct == ryo_values::domain::plan_parts::types::RepeatedRows::Kept
                    && schemas.first().is_some_and(|input| {
                        columns.len() == input.kinds.len()
                            && columns
                                .iter()
                                .enumerate()
                                .all(|(at, column)| column.0 as usize == at)
                    }) =>
                {
                    schemas.first().and_then(|input| input.tree)
                }
                _ => None,
            };
            tree_outputs.push(tree);
            preparations += 1;
            contracts.push(Arc::new(contract));
        }
        rules.push(contracts);
    }
    let request = RequestOperators {
        ordered_existence: plans.iter().map(|_| std::sync::OnceLock::new()).collect(),
        rules,
        preparations: ryo_execution_domain::domain::operator::contract::types::PreparationCount(preparations),
    };
    debug_assert_eq!(
        request.preparations.0,
        plans.iter().map(|plan| plan.steps.len()).sum::<usize>()
    );
    Ok(request)
}

