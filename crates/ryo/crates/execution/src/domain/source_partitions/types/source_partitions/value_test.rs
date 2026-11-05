use super::value::{SourcePartitions, relation_partition};
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{PartitionRequirement, ProducerPartitions},
    file_program::types::{ResolvedObservationBinding, ResolvedObservationProducer},
    operating::errors::OperatingError,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::{
    service::{ALL, provenance_of},
    types::Provenance,
};
use ryo_values::domain::rule_requests::types::RuleParameters;
use ryo_values::domain::{
    hosted::types::HostName,
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationIndex,
        ObservationName, PlanObservation,
    },
    plan_parts::types::{ColumnIndex, ParameterName, StepId, WholeNumber},
    plans::types::{Emit, Operation, ParameterType, PlanParameter, PlanStep},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    runs::types::{RuleApplicability, RuleId},
};
use ryo_values::domain::{
    hosted::types::HostRelation,
    observations::types::{ObservationBindingId, ObservationProducer},
    plan_parts::types::{HostedNumber, Input as PlanInput, ParameterId},
    plans::types::{Plan, StepScope},
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
fn catalog_is_closed_and_uses_actual_provider_origins() {
    assert_eq!(ALL.len(), 69);
    for &relation in ALL {
        let expected = match provenance_of(relation) {
            Provenance::Trees | Provenance::Engine => PartitionRequirement::CurrentFile,
            Provenance::Manifests => PartitionRequirement::Project,
        };
        assert_eq!(relation_partition(relation), expected, "{relation:?}");
    }
}
fn snapshots_are_invariant_but_file_sources_keep_their_requirement() {
    let mut plan = plan();
    plan.parameters.push(PlanParameter {
        name: ParameterName("value".into()),
        kind: ParameterType::Number,
    });
    let sources = SourcePartitions::new(
        &[rule()],
        &[Ok(&plan)],
        &[vec![ParameterBinding::Number(WholeNumber(7))]],
        &[host()],
        &[vec![]],
    )
    .unwrap();
    assert_eq!(
        sources.hosted(RuleIndex(0), HostedNumber(0)),
        Ok(PartitionRequirement::RequestInvariant)
    );
    assert_eq!(
        sources.parameter(RuleIndex(0), ParameterId(0)),
        Ok(PartitionRequirement::RequestInvariant)
    );
    assert_eq!(
        sources.relation(Relation::Function),
        PartitionRequirement::CurrentFile
    );
    assert_eq!(
        sources.relation(Relation::PackageDependency),
        PartitionRequirement::Project
    );
    assert_eq!(
        sources.hosted(RuleIndex(0), HostedNumber(1)),
        Err(OperatingError::NotRead)
    );
    assert_eq!(
        sources.hosted(RuleIndex(1), HostedNumber(0)),
        Err(OperatingError::NotRun)
    );
    assert_eq!(
        sources.parameter(RuleIndex(0), ParameterId(1)),
        Err(OperatingError::Unbound)
    );
    assert!(matches!(
        sources.observation(RuleIndex(0), ObservationBindingId(0)),
        Err(OperatingError::ObservationIncomplete { .. })
    ));
}
fn actual_project_root_is_project_even_for_zero_input_hosted_operation() {
    let (consumer, plan, binding) = observed(StepScope::CrossFile);
    let producer = producer(StepScope::CrossFile);
    let sources = SourcePartitions::new(
        &[consumer, rule()],
        &[Ok(&plan), Ok(&producer)],
        &[vec![], vec![]],
        &[host()],
        &[vec![binding], vec![]],
    )
    .unwrap();
    assert_eq!(
        sources.observation(RuleIndex(0), ObservationBindingId(0)),
        Ok(PartitionRequirement::Project)
    );
}
fn file_observation_retains_its_actual_producer_phase() {
    let (consumer, plan, binding) = observed(StepScope::PerFile);
    let producer = producer(StepScope::PerFile);
    let sources = SourcePartitions::new(
        &[consumer, rule()],
        &[Ok(&plan), Ok(&producer)],
        &[vec![], vec![]],
        &[host()],
        &[vec![binding], vec![]],
    )
    .unwrap();
    assert_eq!(
        sources.observation(RuleIndex(0), ObservationBindingId(0)),
        Ok(PartitionRequirement::CurrentFile)
    );
}
fn stale_output_and_missing_queries_refuse_instead_of_becoming_invariant() {
    let (consumer, plan, mut binding) = observed(StepScope::PerFile);
    binding.producers = vec![ResolvedObservationProducer::Output {
        rule: RuleIndex(1),
        output: ObservationIndex(9),
    }];
    let producer = producer(StepScope::PerFile);
    let sources = SourcePartitions::new(
        &[consumer.clone(), rule()],
        &[Ok(&plan), Ok(&producer)],
        &[vec![], vec![]],
        &[],
        &[vec![binding.clone()], vec![]],
    )
    .unwrap();
    assert!(matches!(
        sources.observation(RuleIndex(0), ObservationBindingId(0)),
        Err(OperatingError::ObservationIncomplete { .. })
    ));
    binding.producers = vec![ResolvedObservationProducer::NoQuery {
        rule: RuleId("absent".into()),
    }];
    let sources = SourcePartitions::new(
        &[consumer, rule()],
        &[Ok(&plan), Ok(&producer)],
        &[vec![], vec![]],
        &[],
        &[vec![binding], vec![]],
    )
    .unwrap();
    assert!(matches!(
        sources.observation(RuleIndex(0), ObservationBindingId(0)),
        Err(OperatingError::ObservationIncomplete { .. })
    ));
}
fn closed_empty_bindings_preserve_schema_and_phase_but_not_mismatches() {
    for (phase, expected) in [
        (StepScope::PerFile, PartitionRequirement::CurrentFile),
        (StepScope::CrossFile, PartitionRequirement::Project),
    ] {
        let (mut consumer, plan, mut binding) = observed(phase);
        consumer.observations[0].producers.clear();
        binding.producers.clear();
        let sources = SourcePartitions::new(
            &[consumer.clone()],
            &[Ok(&plan)],
            &[vec![]],
            &[],
            &[vec![binding.clone()]],
        )
        .unwrap();
        assert_eq!(
            sources.observation(RuleIndex(0), ObservationBindingId(0)),
            Ok(expected)
        );
        consumer.observations.clear();
        let sources =
            SourcePartitions::new(&[consumer], &[Ok(&plan)], &[vec![]], &[], &[vec![binding]])
                .unwrap();
        assert_eq!(
            sources.observation(RuleIndex(0), ObservationBindingId(0)),
            Err(OperatingError::ObservationIncomplete {
                reason: "observation has no authored consumer declaration".into()
            })
        );
    }
}
fn failed_producer_preserves_exact_error_on_demand() {
    let (consumer, plan, binding) = observed(StepScope::PerFile);
    let sources = SourcePartitions::new(
        &[consumer, rule()],
        &[Ok(&plan), Err(OperatingError::WrongBinding)],
        &[vec![], vec![]],
        &[],
        &[vec![binding], vec![]],
    )
    .unwrap();
    assert_eq!(
        sources.observation(RuleIndex(0), ObservationBindingId(0)),
        Err(OperatingError::WrongBinding)
    );
    assert_eq!(
        sources.hosted(RuleIndex(1), HostedNumber(0)),
        Err(OperatingError::WrongBinding)
    );
}
fn malformed_alignment_and_observation_ordinals_are_rejected() {
    assert!(matches!(
        SourcePartitions::new(&[rule()], &[], &[], &[], &[]),
        Err(OperatingError::ShapeMismatch)
    ));
    let (consumer, plan, mut binding) = observed(StepScope::PerFile);
    binding.input = ObservationBindingId(3);
    assert!(matches!(
        SourcePartitions::new(&[consumer], &[Ok(&plan)], &[vec![]], &[], &[vec![binding]]),
        Err(OperatingError::ObservationIncomplete { .. })
    ));
}

fn captured_file_producer_in_project_binding_requires_aggregate() {
    let (consumer, plan, binding) = observed(StepScope::CrossFile);
    let producer = producer(StepScope::PerFile);
    let sources = SourcePartitions::new(
        &[consumer, rule()],
        &[Ok(&plan), Ok(&producer)],
        &[vec![], vec![]],
        &[host()],
        &[vec![binding], vec![]],
    )
    .unwrap();
    assert_eq!(
        sources.observation(RuleIndex(0), ObservationBindingId(0)),
        Ok(PartitionRequirement::Project)
    );
}

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("closed_empty_bindings_preserve_schema_and_phase_but_not_mismatches"),
        word: Word::Succeeds,
        input: || closed_empty_bindings_preserve_schema_and_phase_but_not_mismatches,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("catalog_is_closed_and_uses_actual_provider_origins"),
        word: Word::Succeeds,
        input: || catalog_is_closed_and_uses_actual_provider_origins,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("snapshots_are_invariant_but_file_sources_keep_their_requirement"),
        word: Word::Succeeds,
        input: || snapshots_are_invariant_but_file_sources_keep_their_requirement,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("actual_project_root_is_project_even_for_zero_input_hosted_operation"),
        word: Word::Succeeds,
        input: || actual_project_root_is_project_even_for_zero_input_hosted_operation,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("file_observation_retains_its_actual_producer_phase"),
        word: Word::Succeeds,
        input: || file_observation_retains_its_actual_producer_phase,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("stale_output_and_missing_queries_refuse_instead_of_becoming_invariant"),
        word: Word::Succeeds,
        input: || stale_output_and_missing_queries_refuse_instead_of_becoming_invariant,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("failed_producer_preserves_exact_error_on_demand"),
        word: Word::Succeeds,
        input: || failed_producer_preserves_exact_error_on_demand,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("malformed_alignment_and_observation_ordinals_are_rejected"),
        word: Word::Succeeds,
        input: || malformed_alignment_and_observation_ordinals_are_rejected,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("captured_file_producer_in_project_binding_requires_aggregate"),
        word: Word::Succeeds,
        input: || captured_file_producer_in_project_binding_requires_aggregate,
        expect: Expect::Value(()),
    },
];
fn subject(case: Input) -> Result<Value, Failure> {
    case();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
