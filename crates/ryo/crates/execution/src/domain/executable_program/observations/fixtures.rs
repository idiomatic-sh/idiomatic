//! Frontier coordinates must use the same request-qualified slots as instruction inputs.
use crate::domain::executable_program::types::{PartitionRequirement, ProducerPartitions};
use crate::domain::file_program::types::RequestGraph;
use crate::domain::operating::errors::OperatingError;
use crate::domain::{
    coverage::types::RuleIndex,
    file_program::types::{
        ObservationRequest, ResolvedObservationBinding, ResolvedObservationProducer,
    },
    operator::contract::types::RequestOperators,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{Input, StepId};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::{
    observations::types::{BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName, ObservationName, PlanObservation},
    plan_parts::types::{ColumnIndex, HostedNumber, ParameterId, ObservationBindingId},
    plans::types::Emit,
    stored_facts::types::Digest,
    records::types::{ObservationReadinessLayout, ObservationIndex},
};

pub(crate) struct DeclaredFileProducers;
impl ProducerPartitions for DeclaredFileProducers {
    fn hosted(
        &self,
        _: RuleIndex,
        _: HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn parameter(
        &self,
        _: RuleIndex,
        _: ParameterId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::Unbound)
    }
    fn relation(&self, relation: Relation) -> PartitionRequirement {
        assert!(matches!(
            relation,
            Relation::LanguageTarget | Relation::CompilerFeature
        ));
        PartitionRequirement::CurrentFile
    }
    fn observation(
        &self,
        rule: RuleIndex,
        input: ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        match (rule.0, input.0) {
            (2 | 3, 0) => Ok(PartitionRequirement::CurrentFile),
            _ => Err(OperatingError::ObservationIncomplete {
                reason: format!("undeclared fixture producer {rule:?}/{input:?}").into(),
            }),
        }
    }
}
fn columns(name: ObservationColumnName) -> Vec<ObservationColumn> {
    vec![ObservationColumn {
        name,
        kind: CellKind::File,
    }]
}
fn schema(name: ObservationBindingName, column: ObservationColumnName) -> BoundObservationInput {
    BoundObservationInput {
        name,
        columns: columns(column),
        scope: StepScope::PerFile,
    }
}
fn plan(source: Input) -> Plan {
    Plan {
        observation_inputs: vec![],
        parameters: vec![],
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            operation: Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
            scope: StepScope::PerFile,
        }],
        comparison_origins: vec![],
        observations: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
    }
}
pub(crate) struct Fixture {
    pub(crate) plans: Vec<Plan>,
    pub(crate) graph: RequestGraph,
    pub(crate) operators: RequestOperators,
}
#[derive(Clone, Copy)]
pub(crate) enum FrontierSlots {
    Declared,
    UndeclaredFirst,
}
pub(crate) struct Names {
    pub(crate) left: ObservationBindingName,
    pub(crate) right: ObservationBindingName,
    pub(crate) column: ObservationColumnName,
}
pub(crate) fn fixture(frontier: FrontierSlots, names: Names) -> Fixture {
    let invalid_frontier = matches!(frontier, FrontierSlots::UndeclaredFirst);
    let mut producers = [
        plan(Input::Relation(Relation::LanguageTarget)),
        plan(Input::Relation(Relation::CompilerFeature)),
    ];
    for (index, producer) in producers.iter_mut().enumerate() {
        producer.observations.push(PlanObservation {
            name: ObservationName(format!("producer-{index}").into()),
            source: Input::Step(StepId(0)),
            columns: columns(names.column.clone()),
            scope: StepScope::PerFile,
        });
    }
    let mut consumers = [
        plan(Input::BoundObservation(ObservationBindingId(0))),
        plan(Input::BoundObservation(ObservationBindingId(0))),
    ];
    for (index, consumer) in consumers.iter_mut().enumerate() {
        consumer.observation_inputs.push(schema(
            if index == 0 {
                names.left.clone()
            } else {
                names.right.clone()
            },
            names.column.clone(),
        ));
        consumer
            .frontier
            .push(Input::BoundObservation(ObservationBindingId(
                if invalid_frontier && index == 0 { 1 } else { 0 },
            )));
    }
    let plans: Vec<_> = producers.into_iter().chain(consumers).collect();
    let bindings = vec![
        vec![],
        vec![],
        vec![ResolvedObservationBinding {
            input: ObservationBindingId(0),
            schema: schema(names.left.clone(), names.column.clone()),
            producers: vec![ResolvedObservationProducer::Output {
                rule: RuleIndex(0),
                output: ObservationIndex(0),
            }],
        }],
        vec![ResolvedObservationBinding {
            input: ObservationBindingId(0),
            schema: schema(names.right.clone(), names.column.clone()),
            producers: vec![ResolvedObservationProducer::Output {
                rule: RuleIndex(1),
                output: ObservationIndex(0),
            }],
        }],
    ];
    let operators = crate::application::file_dag::request::prepare(
        &plans,
        &[vec![], vec![], vec![], vec![]],
        &[],
        std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ),
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared_observations(
        &plans,
        &operators,
        ObservationRequest::Bound {
            layout: ObservationReadinessLayout(Digest([7; 16])),
            bindings: &bindings,
        },
    );
    Fixture {
        plans,
        graph,
        operators,
    }
}
