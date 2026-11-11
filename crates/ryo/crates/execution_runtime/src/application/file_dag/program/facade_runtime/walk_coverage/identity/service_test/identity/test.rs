//! Fusion across an owner-issued identity is refused; equal Extends of two rules compute once.
use super::super::super::super::fixtures::operations::{corpus::compile_program, heads::FUNCTION};
use super::fixtures::{
    class, compile, emit_region, equal, extended, keep, logical, plan_of, producer, region_len,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::fusion::types::{
    Barrier, Fusibility, Ineligible,
};
use ryo_relations::domain::relations::service::kinds;
use ryo_values::domain::{
    observations::types::{
        ObservationColumn, ObservationColumnName, ObservationName, PlanObservation,
    },
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, JoinColumn, RepeatedRows, Side, StepId, StepScope,
    },
    plans::types::Operation,
};
use std::collections::HashSet;

fn a_producer_read_once_by_a_pure_projection_is_fused() {
    let program = compile(plan_of(vec![producer(), keep(0)], 1));
    assert_eq!(class(&program, 0), Fusibility::Excluded(Ineligible::Source));
    assert_eq!(class(&program, 1), Fusibility::Fusable);
    assert_eq!(region_len(&program, 1), 2);
}

fn a_handed_over_root_is_not_fused_into() {
    let program = compile(plan_of(vec![producer(), keep(0)], 0));
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Handover));
    assert_eq!(region_len(&program, 1), 1);
}

fn a_frontier_root_is_not_fused_into() {
    let mut plan = plan_of(vec![producer(), keep(0)], 1);
    plan.frontier.push(PlanInput::Step(StepId(0)));
    let program = compile(plan);
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Frontier));
    assert_eq!(region_len(&program, 1), 1);
}

fn an_observation_source_is_not_fused_into() {
    let mut plan = plan_of(vec![producer(), keep(0)], 1);
    plan.observations.push(PlanObservation {
        name: ObservationName("picked".into()),
        source: PlanInput::Step(StepId(0)),
        columns: kinds(FUNCTION)
            .iter()
            .enumerate()
            .map(|(at, kind)| ObservationColumn {
                name: ObservationColumnName(format!("column-{at}").into()),
                kind: *kind,
            })
            .collect(),
        scope: StepScope::PerFile,
    });
    let program = compile(plan);
    assert_eq!(
        class(&program, 1),
        Fusibility::Barrier(Barrier::Observation)
    );
    assert_eq!(region_len(&program, 1), 1);
}

fn a_derived_acquisition_source_is_not_fused_into() {
    let join = Operation::Join {
        left: PlanInput::Relation(FUNCTION),
        right: PlanInput::Step(StepId(0)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
        ],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let program = compile(plan_of(vec![producer(), keep(0), join], 2));
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Derived));
    assert_eq!(region_len(&program, 1), 1);
}

fn a_shared_consumer_keeps_its_producer_publication() {
    let other = Operation::Project {
        from: PlanInput::Step(StepId(0)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let program = compile(plan_of(vec![producer(), keep(0), other], 1));
    assert_eq!(
        class(&program, 1),
        Fusibility::Barrier(Barrier::SharedConsumer)
    );
    assert_eq!(
        class(&program, 2),
        Fusibility::Excluded(Ineligible::Reshapes)
    );
    assert_eq!(region_len(&program, 1), 1);
}

fn a_partition_boundary_is_not_fused_across() {
    let mut plan = plan_of(vec![producer(), keep(0)], 1);
    plan.steps[1].scope = StepScope::CrossFile;
    plan.frontier.push(PlanInput::Step(StepId(0)));
    let program = compile(plan);
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Partition));
    assert_eq!(region_len(&program, 1), 1);
}

fn an_absence_sensitive_producer_publishes_before_a_continuation() {
    let antijoin = Operation::Antijoin {
        left: PlanInput::Relation(FUNCTION),
        right: PlanInput::Relation(FUNCTION),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
    };
    let program = compile(plan_of(vec![antijoin, keep(0)], 1));
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Fault));
    assert_eq!(region_len(&program, 1), 1);
}

fn equivalent_extends_of_two_rules_share_one_region() {
    let program = compile_program(
        &[
            plan_of(vec![producer(), extended(0, 7)], 1),
            plan_of(vec![producer(), extended(0, 7)], 1),
        ],
        &[vec![], vec![]],
    );
    assert_eq!(emit_region(&program, 0), emit_region(&program, 1));
    let shared = &program.regions()[emit_region(&program, 0)];
    assert_eq!(
        shared.consumers().0,
        2,
        "both rules read the one publication"
    );
    assert_eq!(program.regions().len(), 2, "one producer, one extend");
    let producer = program.logical_regions()[logical(&program, 0, 0).0];
    assert_eq!(program.regions()[producer.0].consumers().0, 1);
    // Each rule still names an instruction its region contains.
    for rule in 0..2 {
        let at = logical(&program, rule, 1);
        let region = program.logical_regions()[at.0];
        assert!(
            program.regions()[region.0]
                .instructions()
                .iter()
                .any(|instruction| instruction.logical() == at)
        );
    }
}

fn extends_that_differ_in_operand_or_input_keep_their_own_regions() {
    let other_producer = Operation::Select {
        from: PlanInput::Relation(FUNCTION),
        comparisons: vec![equal(ColumnIndex(0))],
        kept: vec![],
    };
    let program = compile_program(
        &[
            plan_of(vec![producer(), extended(0, 7)], 1),
            plan_of(vec![producer(), extended(0, 8)], 1),
            plan_of(vec![other_producer, extended(0, 7)], 1),
        ],
        &[vec![], vec![], vec![]],
    );
    let regions = (0..3)
        .map(|rule| emit_region(&program, rule))
        .collect::<HashSet<_>>();
    assert_eq!(
        regions.len(),
        3,
        "a different literal or input is never shared"
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    AProducerReadOnceByAPureProjectionIsFused,
    AHandedOverRootIsNotFusedInto,
    AFrontierRootIsNotFusedInto,
    AnObservationSourceIsNotFusedInto,
    ADerivedAcquisitionSourceIsNotFusedInto,
    ASharedConsumerKeepsItsProducerPublication,
    APartitionBoundaryIsNotFusedAcross,
    AnAbsenceSensitiveProducerPublishesBeforeAContinuation,
    EquivalentExtendsOfTwoRulesShareOneRegion,
    ExtendsThatDifferInOperandOrInputKeepTheirOwnRegions,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_producer_read_once_by_a_pure_projection_is_fused"),
        word: Word::Succeeds,
        input: || Scenario::AProducerReadOnceByAPureProjectionIsFused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_handed_over_root_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Scenario::AHandedOverRootIsNotFusedInto,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_frontier_root_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Scenario::AFrontierRootIsNotFusedInto,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_observation_source_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Scenario::AnObservationSourceIsNotFusedInto,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_derived_acquisition_source_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Scenario::ADerivedAcquisitionSourceIsNotFusedInto,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_shared_consumer_keeps_its_producer_publication"),
        word: Word::Succeeds,
        input: || Scenario::ASharedConsumerKeepsItsProducerPublication,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_partition_boundary_is_not_fused_across"),
        word: Word::Succeeds,
        input: || Scenario::APartitionBoundaryIsNotFusedAcross,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_absence_sensitive_producer_publishes_before_a_continuation"),
        word: Word::Succeeds,
        input: || Scenario::AnAbsenceSensitiveProducerPublishesBeforeAContinuation,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("equivalent_extends_of_two_rules_share_one_region"),
        word: Word::Succeeds,
        input: || Scenario::EquivalentExtendsOfTwoRulesShareOneRegion,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("extends_that_differ_in_operand_or_input_keep_their_own_regions"),
        word: Word::Succeeds,
        input: || Scenario::ExtendsThatDifferInOperandOrInputKeepTheirOwnRegions,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::AProducerReadOnceByAPureProjectionIsFused => {
            a_producer_read_once_by_a_pure_projection_is_fused();
            Ok(())
        }
        Scenario::AHandedOverRootIsNotFusedInto => {
            a_handed_over_root_is_not_fused_into();
            Ok(())
        }
        Scenario::AFrontierRootIsNotFusedInto => {
            a_frontier_root_is_not_fused_into();
            Ok(())
        }
        Scenario::AnObservationSourceIsNotFusedInto => {
            an_observation_source_is_not_fused_into();
            Ok(())
        }
        Scenario::ADerivedAcquisitionSourceIsNotFusedInto => {
            a_derived_acquisition_source_is_not_fused_into();
            Ok(())
        }
        Scenario::ASharedConsumerKeepsItsProducerPublication => {
            a_shared_consumer_keeps_its_producer_publication();
            Ok(())
        }
        Scenario::APartitionBoundaryIsNotFusedAcross => {
            a_partition_boundary_is_not_fused_across();
            Ok(())
        }
        Scenario::AnAbsenceSensitiveProducerPublishesBeforeAContinuation => {
            an_absence_sensitive_producer_publishes_before_a_continuation();
            Ok(())
        }
        Scenario::EquivalentExtendsOfTwoRulesShareOneRegion => {
            equivalent_extends_of_two_rules_share_one_region();
            Ok(())
        }
        Scenario::ExtendsThatDifferInOperandOrInputKeepTheirOwnRegions => {
            extends_that_differ_in_operand_or_input_keep_their_own_regions();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
