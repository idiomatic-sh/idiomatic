// ignore: idiomatic.arch/file-length - one case table for the one source-partition table: every row asks one query of a world built by the same fixtures, and the table has no parts of its own to split along.
use super::value::{SourcePartitions, relation_partition};
use crate::domain::{
    coverage::types::RuleIndex,
    observation_schedule::types::{ResolvedObservationBinding, ResolvedObservationProducer},
    operating::errors::OperatingError,
    source_partitions::types::{PartitionRequirement, ProducerPartitions},
};
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::{
    service::{ALL, provenance_of},
    types::Provenance,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
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
use ryo_values::selects;

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
enum World {
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
enum Producer {
    /// A plan of one hosted step and its observation, both of this phase.
    Plan(StepScope),
    /// A plan that failed to compile with this error.
    Failed(OperatingError),
    /// No second rule.
    Absent,
}

/// The consumer's binding.
#[derive(Clone, Copy, Debug)]
enum Binding {
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
enum Query {
    Hosted(RuleIndex, HostedNumber),
    Parameter(RuleIndex, ParameterId),
    Relation(Relation),
    /// The consumer's binding.
    Observation,
}

/// What a row asks.
#[derive(Debug)]
enum Asked {
    /// The partition of every relation of the catalog.
    Catalog,
    /// A world's table and one query of it.
    Of(World, Query),
}

/// The catalog's relations, and how many of them have the partition their provider's origin implies
/// (trees and the engine: the current file; manifests: the project).
#[derive(Debug, PartialEq)]
struct Catalog {
    relations: usize,
    as_their_origin: usize,
}

/// What a row answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Catalog(Catalog),
    Requirement(PartitionRequirement),
}

type Input = Asked;
type Value = Answer;
type Failure = OperatingError;

fn subject(asked: Input) -> Result<Value, Failure> {
    match asked {
        Asked::Catalog => Ok(Answer::Catalog(Catalog {
            relations: ALL.len(),
            as_their_origin: ALL
                .iter()
                .filter(|&&relation| {
                    relation_partition(relation)
                        == match provenance_of(relation) {
                            Provenance::Trees | Provenance::Engine => {
                                PartitionRequirement::CurrentFile
                            }
                            Provenance::Manifests => PartitionRequirement::Project,
                        }
                })
                .count(),
        })),
        Asked::Of(world, query) => ask(world, query).map(Answer::Requirement),
    }
}

/// Builds the world's table and asks it the query.
fn ask(world: World, query: Query) -> Result<PartitionRequirement, OperatingError> {
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
const fn observing(consumer: StepScope, producer: StepScope) -> World {
    World::Observing {
        consumer,
        producer: Producer::Plan(producer),
        binding: Binding::Output(0),
        hosts: true,
    }
}

/// A consumer of this phase whose binding is resolved as given, with a per-file producer and no host
/// relations.
const fn bound(consumer: StepScope, binding: Binding, producer: Producer) -> World {
    World::Observing {
        consumer,
        producer,
        binding,
        hosts: false,
    }
}

/// Whether an error is an incomplete observation for this reason.
fn incomplete(error: &OperatingError, why: &str) -> bool {
    matches!(error, OperatingError::ObservationIncomplete { reason } if reason == why)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("closed_empty_bindings_preserve_schema_and_phase_but_not_mismatches"),
        word: Word::Succeeds,
        input: || {
            let world = bound(StepScope::PerFile, Binding::ClosedEmpty, Producer::Absent);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::CurrentFile)),
    },
    Case {
        name: CaseName("a_closed_empty_project_binding_is_project"),
        word: Word::Succeeds,
        input: || {
            let world = bound(StepScope::CrossFile, Binding::ClosedEmpty, Producer::Absent);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::Project)),
    },
    Case {
        name: CaseName("an_undeclared_closed_empty_file_binding_refuses"),
        word: Word::Refuses,
        input: || {
            let world = bound(StepScope::PerFile, Binding::Undeclared, Producer::Absent);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Refusal(|error| {
            selects!(incomplete(
                error,
                "observation has no authored consumer declaration"
            ))
        }),
    },
    Case {
        name: CaseName("an_undeclared_closed_empty_project_binding_refuses"),
        word: Word::Refuses,
        input: || {
            let world = bound(StepScope::CrossFile, Binding::Undeclared, Producer::Absent);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Refusal(|error| {
            selects!(incomplete(
                error,
                "observation has no authored consumer declaration"
            ))
        }),
    },
    Case {
        name: CaseName("catalog_is_closed_and_uses_actual_provider_origins"),
        word: Word::Succeeds,
        input: || Asked::Catalog,
        expect: Expect::Value(Answer::Catalog(Catalog {
            relations: 69,
            as_their_origin: 69,
        })),
    },
    Case {
        name: CaseName("snapshots_are_invariant_but_file_sources_keep_their_requirement"),
        word: Word::Succeeds,
        input: || {
            Asked::Of(
                World::Snapshot,
                Query::Hosted(RuleIndex(0), HostedNumber(0)),
            )
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::RequestInvariant)),
    },
    Case {
        name: CaseName("a_bound_parameter_is_invariant"),
        word: Word::Succeeds,
        input: || {
            Asked::Of(
                World::Snapshot,
                Query::Parameter(RuleIndex(0), ParameterId(0)),
            )
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::RequestInvariant)),
    },
    Case {
        name: CaseName("a_tree_relation_is_the_current_file"),
        word: Word::Succeeds,
        input: || Asked::Of(World::Snapshot, Query::Relation(Relation::Function)),
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::CurrentFile)),
    },
    Case {
        name: CaseName("a_manifest_relation_is_the_project"),
        word: Word::Succeeds,
        input: || {
            Asked::Of(
                World::Snapshot,
                Query::Relation(Relation::PackageDependency),
            )
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::Project)),
    },
    Case {
        name: CaseName("an_unknown_host_relation_refuses"),
        word: Word::Refuses,
        input: || {
            Asked::Of(
                World::Snapshot,
                Query::Hosted(RuleIndex(0), HostedNumber(1)),
            )
        },
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("a_host_relation_of_an_unknown_rule_refuses"),
        word: Word::Refuses,
        input: || {
            Asked::Of(
                World::Snapshot,
                Query::Hosted(RuleIndex(1), HostedNumber(0)),
            )
        },
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("an_unbound_parameter_refuses"),
        word: Word::Refuses,
        input: || {
            Asked::Of(
                World::Snapshot,
                Query::Parameter(RuleIndex(0), ParameterId(1)),
            )
        },
        expect: Expect::Error(OperatingError::Unbound),
    },
    Case {
        name: CaseName("an_observation_the_rule_does_not_declare_refuses"),
        word: Word::Refuses,
        input: || Asked::Of(World::Snapshot, Query::Observation),
        expect: Expect::Refusal(|error| {
            selects!(incomplete(
                error,
                "observation input has no producer declaration"
            ))
        }),
    },
    Case {
        name: CaseName("actual_project_root_is_project_even_for_zero_input_hosted_operation"),
        word: Word::Succeeds,
        input: || {
            let world = observing(StepScope::CrossFile, StepScope::CrossFile);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::Project)),
    },
    Case {
        name: CaseName("file_observation_retains_its_actual_producer_phase"),
        word: Word::Succeeds,
        input: || {
            let world = observing(StepScope::PerFile, StepScope::PerFile);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::CurrentFile)),
    },
    Case {
        name: CaseName("stale_output_and_missing_queries_refuse_instead_of_becoming_invariant"),
        word: Word::Refuses,
        input: || {
            let file = Producer::Plan(StepScope::PerFile);
            let world = bound(StepScope::PerFile, Binding::Output(9), file);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Refusal(|error| {
            selects!(incomplete(
                error,
                "observation producer output ordinal is undeclared"
            ))
        }),
    },
    Case {
        name: CaseName("a_binding_to_a_rule_with_no_query_refuses"),
        word: Word::Refuses,
        input: || {
            let file = Producer::Plan(StepScope::PerFile);
            let world = bound(StepScope::PerFile, Binding::NoQuery, file);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Refusal(|error| {
            selects!(incomplete(
                error,
                "observation producer has no executable query"
            ))
        }),
    },
    Case {
        name: CaseName("failed_producer_preserves_exact_error_on_demand"),
        word: Word::Refuses,
        input: || {
            let failed = Producer::Failed(OperatingError::WrongBinding);
            let world = bound(StepScope::PerFile, Binding::Output(0), failed);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("a_failed_producer_refuses_its_host_relations_with_its_error"),
        word: Word::Refuses,
        input: || {
            let failed = Producer::Failed(OperatingError::WrongBinding);
            let world = bound(StepScope::PerFile, Binding::Output(0), failed);
            Asked::Of(world, Query::Hosted(RuleIndex(1), HostedNumber(0)))
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("malformed_alignment_and_observation_ordinals_are_rejected"),
        word: Word::Refuses,
        input: || Asked::Of(World::Misaligned, Query::Observation),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("a_misnumbered_binding_is_rejected"),
        word: Word::Refuses,
        input: || {
            let world = bound(StepScope::PerFile, Binding::Misnumbered, Producer::Absent);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Refusal(|error| {
            selects!(incomplete(
                error,
                "observation binding ordinal is not its declared slot"
            ))
        }),
    },
    Case {
        name: CaseName("captured_file_producer_in_project_binding_requires_aggregate"),
        word: Word::Succeeds,
        input: || {
            let world = observing(StepScope::CrossFile, StepScope::PerFile);
            Asked::Of(world, Query::Observation)
        },
        expect: Expect::Value(Answer::Requirement(PartitionRequirement::Project)),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
