//! Prepare every operation once for one request, shared by all file workers.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::operator::service::registered;
use crate::domain::table_cells::types::TreeForm;
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

use crate::domain::operator::contract::types::RequestOperators;

impl RequestOperators {
    pub(crate) fn prepare(
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        hosted: &[HostRelation],
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Result<Self, OperatingError> {
        Self::prepare_with_schemas(plans, bindings, hosted, &kinds, semantic)
    }
    pub(crate) fn prepare_with_schemas<'s>(
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        hosted: &[HostRelation],
        relation_kinds: &dyn Fn(Relation) -> &'s [CellKind],
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Result<Self, OperatingError> {
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
        let mut views = super::derived::request::RequestViews::new(Arc::clone(&semantic));
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
                                tree: crate::domain::table_cells::service::native_form(*relation),
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
                                    .and_then(|contract| contract.schema.as_ref().ok())
                                    .map_or(&[], Vec::as_slice),
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
                let mut contract = registered(kind_of(&step.operation)).prepare(
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
        let selection_plans = Arc::new(
            plans
                .iter()
                .enumerate()
                .map(|(rule, plan)| {
                    crate::domain::stored_selection::types::program::value::SelectionProgram::new(
                        plan,
                        bindings.get(rule).map_or(&[], Vec::as_slice),
                    )
                    .prepare()
                })
                .collect::<Vec<_>>(),
        );
        let selection_views = super::derived::selection::prepare_views_with_schemas(
            selection_plans
                .iter()
                .flat_map(|plan| plan.steps.iter().flatten())
                .cloned(),
            relation_kinds,
            semantic,
        )?;
        let physical=plans.iter().enumerate().map(|(rule,plan)| {
            let selectors = &selection_plans[rule];
            plan.steps.iter().enumerate().map(|(at,_step)| {
                crate::domain::table_model::collections::representations::publication::PreparedPhysical{selector:selectors.steps.get(at).cloned().flatten()}
            }).collect()
        }).collect();
        let ordered_existence = Arc::new(
            plans
                .iter()
                .enumerate()
                .map(|(rule, plan)| {
                    crate::domain::ordered_existence::service::compile(plan, &|id| {
                        rules
                            .get(rule)?
                            .get(id.0 as usize)?
                            .schema
                            .as_ref()
                            .ok()
                            .map(Vec::as_slice)
                    })
                })
                .collect(),
        );
        let request = Self {
            ordered_existence,
            selection_plans,
            selection_views,
            rules,
            preparations,
            physical,
        };
        debug_assert_eq!(
            request.preparations,
            plans.iter().map(|plan| plan.steps.len()).sum::<usize>()
        );
        Ok(request)
    }
}
