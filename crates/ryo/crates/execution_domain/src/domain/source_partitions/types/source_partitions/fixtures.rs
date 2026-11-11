//! The worlds the source-partition cases build a table over, and the one query each asks of it.
use super::value::SourcePartitions;
use crate::domain::{
    coverage::types::RuleIndex,
    observation_schedule::types::{ResolvedObservationBinding, ResolvedObservationProducer},
    operating::errors::OperatingError,
    source_partitions::types::{PartitionRequirement, ProducerPartitions},
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rule_requests::types::RuleParameters;
use ryo_values::domain::{
    hosted::types::HostName,
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationName,
        PlanObservation,
    },
    plan_parts::types::{ColumnIndex, ParameterName, StepId, WholeNumber},
    plans::types::{Emit, Operation, PlanStep},
    query_bindings::types::{ParameterType, PlanParameter},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    records::types::ObservationIndex,
    runs::types::{RuleApplicability, RuleId},
};
use ryo_values::domain::{
    hosted::types::HostRelation,
    plan_parts::types::{
        HostedNumber, Input as PlanInput, ObservationBindingId, ParameterId, StepScope,
    },
    plans::types::Plan,
    query_bindings::types::ParameterBinding,
    rule_requests::types::RuleSpec,
};

fn rule() -> RuleSpec {
    RuleSpec {
        id: RuleId("consumer".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("root".into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}
fn plan() -> Plan {
    Plan {
        observation_inputs: vec![],
        parameters: vec![],
        steps: vec![],
        comparison_origins: vec![],
        observations: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
fn host() -> HostRelation {
    HostRelation {
        name: HostName("snapshot".into()),
        columns: vec![],
        rows: vec![],
    }
}
fn observed(phase: StepScope) -> (RuleSpec, Plan, ResolvedObservationBinding) {
    let name = ObservationBindingName("bound".into());
    let mut rule = rule();
    rule.observations.push(ObservationBinding {
        name: name.clone(),
        columns: vec![],
        producers: vec![
            ryo_values::domain::observations::types::ObservationProducer::Output {
                rule: RuleId("consumer".into()),
                output: ObservationName("root".into()),
            },
        ],
    });
    let schema = BoundObservationInput {
        name,
        columns: vec![],
        scope: phase,
    };
    let mut plan = plan();
    plan.observation_inputs.push(schema.clone());
    let binding = ResolvedObservationBinding {
        input: ObservationBindingId(0),
        schema,
        producers: vec![ResolvedObservationProducer::Output {
            rule: RuleIndex(1),
            output: ObservationIndex(0),
        }],
    };
    (rule, plan, binding)
}
fn producer(phase: StepScope) -> Plan {
    let mut plan = plan();
    plan.steps.push(PlanStep {
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
        scope: phase,
    });
    plan.observations.push(PlanObservation {
        name: ObservationName("root".into()),
        source: PlanInput::Step(StepId(0)),
        columns: vec![],
        scope: phase,
    });
    plan
}
/// The world a source-partition table is built over.
#[derive(Debug)]
pub(super) enum World {
    /// One rule with no observation, a number parameter bound to 7, and one host relation.
    Snapshot,
    /// A consumer rule of this phase observing through one binding, a producer rule, and host
    /// relations or none.
    Observing {
        consumer: StepScope,
        producer: Producer,
        binding: Binding,
        hosts: bool,
    },
    /// One rule and no plan.
    Misaligned,
}

/// The second rule of an observing world, whose `root` output the consumer observes.
#[derive(Debug)]
pub(super) enum Producer {
    /// A plan of one hosted step and its observation, both of this phase.
    Plan(StepScope),
    /// A plan that failed to compile with this error.
    Failed(OperatingError),
    /// No second rule.
    Absent,
}

/// The consumer's binding.
#[derive(Clone, Copy, Debug)]
pub(super) enum Binding {
    /// Resolved to the producer's output at this index.
    Output(u32),
    /// Resolved to a rule that has no query.
    NoQuery,
    /// Declared and resolved with no producer.
    ClosedEmpty,
    /// Resolved with no producer, and not declared by the consumer.
    Undeclared,
    /// Resolved to the producer's output 0 as the binding at ordinal 3.
    Misnumbered,
}

/// What a row asks of a world's table.
#[derive(Clone, Copy, Debug)]
pub(super) enum Query {
    Hosted(RuleIndex, HostedNumber),
    Parameter(RuleIndex, ParameterId),
    Relation(Relation),
    /// The consumer's binding.
    Observation,
}
/// Builds the world's table and asks it the query.
pub(super) fn ask(world: World, query: Query) -> Result<PartitionRequirement, OperatingError> {
    let (rules, plans, parameters, hosts, bindings) = match world {
        World::Snapshot => {
            let mut plan = plan();
            plan.parameters.push(PlanParameter {
                name: ParameterName("value".into()),
                kind: ParameterType::Number,
            });
            (
                vec![rule()],
                vec![Ok(plan)],
                vec![vec![ParameterBinding::Number(WholeNumber(7))]],
                vec![host()],
                vec![vec![]],
            )
        }
        World::Misaligned => (vec![rule()], vec![], vec![], vec![], vec![]),
        World::Observing {
            consumer,
            producer: second,
            binding: resolved,
            hosts,
        } => {
            let (mut consumer, plan, mut binding) = observed(consumer);
            match resolved {
                Binding::Output(output) => {
                    binding.producers = vec![ResolvedObservationProducer::Output {
                        rule: RuleIndex(1),
                        output: ObservationIndex(output),
                    }];
                }
                Binding::NoQuery => {
                    binding.producers = vec![ResolvedObservationProducer::NoQuery {
                        rule: RuleId("absent".into()),
                    }];
                }
                Binding::ClosedEmpty => {
                    consumer.observations[0].producers.clear();
                    binding.producers.clear();
                }
                Binding::Undeclared => {
                    consumer.observations.clear();
                    binding.producers.clear();
                }
                Binding::Misnumbered => binding.input = ObservationBindingId(3),
            }
            let mut rules = vec![consumer];
            let mut plans = vec![Ok(plan)];
            let mut bindings = vec![vec![binding]];
            let produced = match second {
                Producer::Plan(phase) => Some(Ok(producer(phase))),
                Producer::Failed(error) => Some(Err(error)),
                Producer::Absent => None,
            };
            if let Some(produced) = produced {
                rules.push(rule());
                plans.push(produced);
                bindings.push(vec![]);
            }
            let parameters = vec![vec![]; rules.len()];
            let hosts = if hosts { vec![host()] } else { vec![] };
            (rules, plans, parameters, hosts, bindings)
        }
    };
    let plans: Vec<Result<&Plan, OperatingError>> = plans
        .iter()
        .map(|plan| plan.as_ref().map_err(Clone::clone))
        .collect();
    let sources = SourcePartitions::new(&rules, &plans, &parameters, &hosts, &bindings)?;
    match query {
        Query::Hosted(rule, number) => sources.hosted(rule, number),
        Query::Parameter(rule, id) => sources.parameter(rule, id),
        Query::Relation(relation) => Ok(sources.relation(relation)),
        Query::Observation => sources.observation(RuleIndex(0), ObservationBindingId(0)),
    }
}

/// A consumer of this phase observing the producer's output 0, with the producer and host relations.
pub(super) const fn observing(consumer: StepScope, producer: StepScope) -> World {
    World::Observing {
        consumer,
        producer: Producer::Plan(producer),
        binding: Binding::Output(0),
        hosts: true,
    }
}

/// A consumer of this phase whose binding is resolved as given, with a per-file producer and no host
/// relations.
pub(super) const fn bound(consumer: StepScope, binding: Binding, producer: Producer) -> World {
    World::Observing {
        consumer,
        producer,
        binding,
        hosts: false,
    }
}

/// Whether an error is an incomplete observation for this reason.
pub(super) fn incomplete(error: &OperatingError, why: &str) -> bool {
    matches!(error, OperatingError::ObservationIncomplete { reason } if reason == why)
}
