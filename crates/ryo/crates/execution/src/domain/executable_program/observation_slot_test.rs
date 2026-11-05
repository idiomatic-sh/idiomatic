//! Frontier coordinates must use the same request-qualified slots as instruction inputs.
use super::*;
use crate::domain::{
    coverage::types::RuleIndex,
    file_program::types::{
        ObservationRequest, ResolvedObservationBinding, ResolvedObservationProducer,
    },
    operator::contract::types::RequestOperators,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::{
    observations::types::{
        BoundObservationSchema, ObservationBindingId, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationIndex, ObservationName, ObservationReadinessLayout,
        PlanObservation,
    },
    plan_parts::types::{ColumnIndex, HostedNumber, ParameterId},
    plans::types::Emit,
    stored_facts::types::Digest,
};

struct DeclaredFileProducers;
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
                reason: "undeclared fixture producer".into(),
            }),
        }
    }
}
fn columns() -> Vec<ObservationColumn> {
    vec![ObservationColumn {
        name: ObservationColumnName("file".into()),
        kind: CellKind::File,
    }]
}
fn schema(name: &str) -> BoundObservationSchema {
    BoundObservationSchema {
        name: ObservationBindingName(name.into()),
        columns: columns(),
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
struct Fixture {
    plans: Vec<Plan>,
    graph: RequestGraph,
    operators: RequestOperators,
}
fn fixture(invalid_frontier: bool) -> Fixture {
    let mut producers = [
        plan(Input::Relation(Relation::LanguageTarget)),
        plan(Input::Relation(Relation::CompilerFeature)),
    ];
    for (index, producer) in producers.iter_mut().enumerate() {
        producer.observations.push(PlanObservation {
            name: ObservationName(format!("producer-{index}").into()),
            source: Input::Step(StepId(0)),
            columns: columns(),
            scope: StepScope::PerFile,
        });
    }
    let mut consumers = [
        plan(Input::BoundObservation(ObservationBindingId(0))),
        plan(Input::BoundObservation(ObservationBindingId(0))),
    ];
    for (index, consumer) in consumers.iter_mut().enumerate() {
        consumer
            .observation_inputs
            .push(schema(if index == 0 { "left" } else { "right" }));
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
            schema: schema("left"),
            producers: vec![ResolvedObservationProducer::Output {
                rule: RuleIndex(0),
                output: ObservationIndex(0),
            }],
        }],
        vec![ResolvedObservationBinding {
            input: ObservationBindingId(0),
            schema: schema("right"),
            producers: vec![ResolvedObservationProducer::Output {
                rule: RuleIndex(1),
                output: ObservationIndex(0),
            }],
        }],
    ];
    let batches = vec![
        vec![RuleIndex(0), RuleIndex(1)],
        vec![RuleIndex(2), RuleIndex(3)],
    ];
    let operators = RequestOperators::prepare(
        &plans,
        &[vec![], vec![], vec![], vec![]],
        &[],
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
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
#[test]
fn two_local_zero_inputs_keep_their_distinct_request_producers() {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(false);
    let metadata = graph.observations.as_ref().unwrap();
    assert_eq!(metadata.slots.len(), 2);
    assert_eq!(metadata.slots[0].rule, 2);
    assert_eq!(metadata.slots[1].rule, 3);
    assert_eq!(
        metadata.bindings[2][0].input.producers[0],
        ResolvedObservationProducer::Output {
            rule: RuleIndex(0),
            output: ObservationIndex(0)
        }
    );
    assert_eq!(
        metadata.bindings[3][0].input.producers[0],
        ResolvedObservationProducer::Output {
            rule: RuleIndex(1),
            output: ObservationIndex(0)
        }
    );
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators).unwrap();
    assert_eq!(
        program.rules[2].frontier[0],
        Ok(InputSource::Observation(0))
    );
    assert_eq!(
        program.rules[3].frontier[0],
        Ok(InputSource::Observation(1))
    );
}
#[test]
fn undeclared_frontier_ordinal_is_a_typed_refusal() {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(true);
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators).unwrap();
    assert!(matches!(
        &program.rules[2].frontier[0],
        Err(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(
        program.rules[3].frontier[0],
        Ok(InputSource::Observation(1))
    );
}

#[test]
fn declared_local_input_without_its_qualified_slot_is_a_typed_refusal() {
    let Fixture {
        plans,
        mut graph,
        operators,
    } = fixture(false);
    graph.observations.as_mut().unwrap().slots[0].rule = 99;
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators).unwrap();
    assert!(matches!(
        &program.rules[2].frontier[0],
        Err(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(
        program.rules[3].frontier[0],
        Ok(InputSource::Observation(1))
    );
}
