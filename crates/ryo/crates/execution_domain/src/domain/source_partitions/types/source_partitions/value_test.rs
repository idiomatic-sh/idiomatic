//! The source-partition table: each relation, host relation, parameter and observation binding
//! answers the partition its rows come from, and a table that cannot answer refuses.
use super::fixtures::{Binding, Producer, Query, World, ask, bound, incomplete, observing};
use super::value::relation_partition;
use crate::domain::{
    coverage::types::RuleIndex, operating::errors::OperatingError,
    source_partitions::types::PartitionRequirement,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::{
    service::{ALL, provenance_of},
    types::Provenance,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{HostedNumber, ParameterId, StepScope};
use ryo_values::selects;

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
