//! The `operators` service: free functions named by verb over the domain's types, with no IO and no ports.

use crate::domain::file_program::types::RequestOperators;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::operator::service::prepare_operator;
use ryo_operators::domain::tree_cells::types::TreeForm;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::hosted::types::{HostKind, HostRelation};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::service::{inputs_of, kind_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

/// Prepares every step of every plan against the schemas `relation_kinds` answers, handing each contract to `bind`
/// for its derived views before it is kept.
///
/// # Errors
///
/// The first error preparing a step's contract returns, or the error `bind` returns for a contract.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::file_program::service::operators::service::prepare;
/// ```
pub fn prepare<'s>(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[HostRelation],
    relation_kinds: &dyn Fn(Relation) -> &'s [CellKind],
    bind: &mut dyn FnMut(&mut OperatorContract) -> Result<(), OperatingError>,
) -> Result<RequestOperators, OperatingError> {
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
                            tree: ryo_operators::domain::tree_cells::service::native_form(*relation),
                        },
                        Input::BoundObservation(id) => InputShape {
                            kinds: observed_kinds
                                .get(usize::try_from(id.0).map_err(|_| OperatingError::NotRun)?)
                                .ok_or(OperatingError::NotRun)?,
                            tree: None,
                        },
                        Input::Step(id) => InputShape {
                            kinds: contracts.get(id.0 as usize).map_or(&[], |contract| {
                                match &contract.schema {
                                    Ok(kinds) => kinds.as_slice(),
                                    Err(_) => &[],
                                }
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
                step.scope,
            )?;
            bind(&mut contract)?;
            let tree = crate::domain::operation_schema::projection::service::tree_output(
                &step.operation,
                schemas.first().copied(),
            );
            tree_outputs.push(tree);
            preparations += 1;
            contracts.push(Arc::new(contract));
        }
        rules.push(contracts);
    }
    let request = RequestOperators {
        ordered_existence: plans.iter().map(|_| std::sync::OnceLock::new()).collect(),
        rules,
        preparations: crate::domain::operator::contract::types::PreparationCount(preparations),
    };
    debug_assert_eq!(
        request.preparations.0,
        plans.iter().map(|plan| plan.steps.len()).sum::<usize>()
    );
    Ok(request)
}
