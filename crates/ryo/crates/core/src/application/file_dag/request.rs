//! Prepare every operation once for one request, shared by all file workers.
use crate::domain::features::service::kind_of;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::operator::service::registered;
use crate::domain::table_cells::types::TreeForm;
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::{
    service::kinds,
    types::{CellKind, Relation},
};
use ryo_values::domain::hosted::types::{HostKind, HostRelation};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::{
    service::{inputs_of, named},
    types::{Operation, Plan},
};
use std::sync::Arc;

use crate::domain::operator::contract::types::RequestOperators;

impl RequestOperators {
    pub(crate) fn prepare(
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        hosted: &[HostRelation],
    ) -> Result<Self, OperatingError> {
        Self::prepare_with_schemas(plans, bindings, hosted, &kinds)
    }
    pub(crate) fn prepare_with_schemas<'s>(
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        hosted: &[HostRelation],
        relation_kinds: &dyn Fn(Relation) -> &'s [CellKind],
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
        let mut rules = Vec::with_capacity(plans.len());
        let mut preparations = 0;
        for (rule, plan) in plans.iter().enumerate() {
            let mut contracts: Vec<Arc<OperatorContract>> = Vec::with_capacity(plan.steps.len());
            for step in &plan.steps {
                let dependencies = inputs_of(&step.operation);
                let mut schemas = named(&dependencies)
                    .iter()
                    .map(|input| match input {
                        Input::Relation(relation) => InputSchema {
                            kinds: relation_kinds(*relation),
                            tree: match relation {
                                Relation::SyntaxTree => Some(TreeForm::Elements),
                                Relation::SyntaxChild => Some(TreeForm::Edges),
                                _ => None,
                            },
                        },
                        Input::Step(id) => InputSchema {
                            kinds: contracts
                                .get(id.0 as usize)
                                .and_then(|contract| contract.schema.as_ref().ok())
                                .map_or(&[], Vec::as_slice),
                            tree: None,
                        },
                    })
                    .collect::<Vec<_>>();
                // Hosted is a real request source. Its schema is not a pretend
                // dependency and no evaluator special-case may bypass registration.
                if let Operation::Hosted { relation } = &step.operation {
                    schemas.push(InputSchema {
                        kinds: host_kinds
                            .get(relation.0 as usize)
                            .map_or(&[], Vec::as_slice),
                        tree: None,
                    });
                }
                let contract = registered(kind_of(&step.operation)).prepare(
                    &step.operation,
                    &schemas,
                    bindings.get(rule).map_or(&[], Vec::as_slice),
                )?;
                preparations += 1;
                contracts.push(Arc::new(contract));
            }
            rules.push(contracts);
        }
        let request = Self {
            rules,
            preparations,
        };
        debug_assert_eq!(
            request.preparations,
            plans.iter().map(|plan| plan.steps.len()).sum::<usize>()
        );
        Ok(request)
    }
}
