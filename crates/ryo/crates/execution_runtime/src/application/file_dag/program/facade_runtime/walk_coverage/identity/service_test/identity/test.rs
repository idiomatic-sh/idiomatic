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

/// The rule program compiled around a producer and its continuation.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// The producer, read once by a pure projection that is emitted.
    ReadOnce,
    /// The same, with the producer emitted: its root is handed over.
    HandedOver,
    /// The same, with the producer a frontier.
    Frontier,
    /// The same, with the producer a named observation.
    Observed,
    /// The same, followed by a join that acquires the projection.
    JoinedAfter,
    /// The same, with a second, distinct projection of the producer.
    SecondReader,
    /// The same, with the projection across files and the producer a frontier.
    CrossFile,
    /// A projection of an antijoin.
    AfterAntijoin,
    /// One rule per entry: an Extend of this producer (0 the plain, 1 a selection) by this number.
    Extends(&'static [(u8, i64)]),
}

/// How the continuation was compiled.
#[derive(Debug, PartialEq)]
enum Fused {
    /// The continuation's class, its region's instructions, and the class of the other step it
    /// is shown beside (the producer, or a second reader).
    Continuation {
        class: Fusibility,
        region: usize,
        beside: Option<Fusibility>,
    },
    /// The rules share one emit region: its consumers, the program's regions, the producer's
    /// consumers, and the rules whose emit instruction is in its region.
    Shared {
        emit_consumers: usize,
        regions: usize,
        producer_consumers: usize,
        rules_in_their_regions: usize,
    },
    /// The rules' distinct emit regions, and the rules whose emit instruction is in its region.
    Separate {
        distinct: usize,
        rules_in_their_regions: usize,
    },
}

const fn continuation(class: Fusibility, region: usize, beside: Option<Fusibility>) -> Fused {
    Fused::Continuation {
        class,
        region,
        beside,
    }
}

type Input = Shape;
type Value = Fused;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_producer_read_once_by_a_pure_projection_is_fused"),
        word: Word::Succeeds,
        input: || Shape::ReadOnce,
        expect: Expect::Value(continuation(
            Fusibility::Fusable,
            2,
            Some(Fusibility::Excluded(Ineligible::Source)),
        )),
    },
    Case {
        name: CaseName("a_handed_over_root_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Shape::HandedOver,
        expect: Expect::Value(continuation(
            Fusibility::Barrier(Barrier::Handover),
            1,
            None,
        )),
    },
    Case {
        name: CaseName("a_frontier_root_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Shape::Frontier,
        expect: Expect::Value(continuation(
            Fusibility::Barrier(Barrier::Frontier),
            1,
            None,
        )),
    },
    Case {
        name: CaseName("an_observation_source_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Shape::Observed,
        expect: Expect::Value(continuation(
            Fusibility::Barrier(Barrier::Observation),
            1,
            None,
        )),
    },
    Case {
        name: CaseName("a_derived_acquisition_source_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Shape::JoinedAfter,
        expect: Expect::Value(continuation(Fusibility::Barrier(Barrier::Derived), 1, None)),
    },
    Case {
        name: CaseName("a_shared_consumer_keeps_its_producer_publication"),
        word: Word::Succeeds,
        input: || Shape::SecondReader,
        expect: Expect::Value(continuation(
            Fusibility::Barrier(Barrier::SharedConsumer),
            1,
            Some(Fusibility::Excluded(Ineligible::Reshapes)),
        )),
    },
    Case {
        name: CaseName("a_partition_boundary_is_not_fused_across"),
        word: Word::Succeeds,
        input: || Shape::CrossFile,
        expect: Expect::Value(continuation(
            Fusibility::Barrier(Barrier::Partition),
            1,
            None,
        )),
    },
    Case {
        name: CaseName("an_absence_sensitive_producer_publishes_before_a_continuation"),
        word: Word::Succeeds,
        input: || Shape::AfterAntijoin,
        expect: Expect::Value(continuation(Fusibility::Barrier(Barrier::Fault), 1, None)),
    },
    Case {
        name: CaseName("equivalent_extends_of_two_rules_share_one_region"),
        word: Word::Succeeds,
        input: || Shape::Extends(&[(0, 7), (0, 7)]),
        expect: Expect::Value(Fused::Shared {
            emit_consumers: 2,
            regions: 2,
            producer_consumers: 1,
            rules_in_their_regions: 2,
        }),
    },
    Case {
        name: CaseName("extends_that_differ_in_operand_or_input_keep_their_own_regions"),
        word: Word::Succeeds,
        input: || Shape::Extends(&[(0, 7), (0, 8), (1, 7)]),
        expect: Expect::Value(Fused::Separate {
            distinct: 3,
            rules_in_their_regions: 3,
        }),
    },
];

/// Compiles the shape and reads how its continuation was classified and placed.
fn subject(input: Input) -> Result<Value, Failure> {
    let program = match input {
        Shape::ReadOnce => compile(plan_of(vec![producer(), keep(0)], 1)),
        Shape::HandedOver => compile(plan_of(vec![producer(), keep(0)], 0)),
        Shape::Frontier => {
            let mut plan = plan_of(vec![producer(), keep(0)], 1);
            plan.frontier.push(PlanInput::Step(StepId(0)));
            compile(plan)
        }
        Shape::Observed => {
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
            compile(plan)
        }
        Shape::JoinedAfter => {
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
            compile(plan_of(vec![producer(), keep(0), join], 2))
        }
        Shape::SecondReader => {
            let other = Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            };
            compile(plan_of(vec![producer(), keep(0), other], 1))
        }
        Shape::CrossFile => {
            let mut plan = plan_of(vec![producer(), keep(0)], 1);
            plan.steps[1].scope = StepScope::CrossFile;
            plan.frontier.push(PlanInput::Step(StepId(0)));
            compile(plan)
        }
        Shape::AfterAntijoin => {
            let antijoin = Operation::Antijoin {
                left: PlanInput::Relation(FUNCTION),
                right: PlanInput::Relation(FUNCTION),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            };
            compile(plan_of(vec![antijoin, keep(0)], 1))
        }
        Shape::Extends(extends) => return Ok(regions(extends)),
    };
    let beside = match input {
        Shape::ReadOnce => Some(class(&program, 0)),
        Shape::SecondReader => Some(class(&program, 2)),
        Shape::HandedOver
        | Shape::Frontier
        | Shape::Observed
        | Shape::JoinedAfter
        | Shape::CrossFile
        | Shape::AfterAntijoin
        | Shape::Extends(_) => None,
    };
    Ok(Fused::Continuation {
        class: class(&program, 1),
        region: region_len(&program, 1),
        beside,
    })
}

/// Compiles one rule per Extend and reads how their emit regions are shared.
fn regions(extends: &[(u8, i64)]) -> Fused {
    let plans = extends
        .iter()
        .map(|&(source, number)| {
            let head = if source == 0 {
                producer()
            } else {
                Operation::Select {
                    from: PlanInput::Relation(FUNCTION),
                    comparisons: vec![equal(ColumnIndex(0))],
                    kept: vec![],
                }
            };
            plan_of(vec![head, extended(0, number)], 1)
        })
        .collect::<Vec<_>>();
    let program = compile_program(&plans, &vec![vec![]; plans.len()]);
    let emits = (0..plans.len())
        .map(|rule| emit_region(&program, rule))
        .collect::<HashSet<_>>();
    // Each rule still names an instruction its region contains.
    let rules_in_their_regions = (0..plans.len())
        .filter(|&rule| {
            let at = logical(&program, rule, 1);
            let region = program.logical_regions()[at.0];
            program.regions()[region.0]
                .instructions()
                .iter()
                .any(|instruction| instruction.logical() == at)
        })
        .count();
    if emits.len() == 1 {
        // Both rules read the one publication: one producer, one extend.
        let producer = program.logical_regions()[logical(&program, 0, 0).0];
        Fused::Shared {
            emit_consumers: program.regions()[emit_region(&program, 0)].consumers().0,
            regions: program.regions().len(),
            producer_consumers: program.regions()[producer.0].consumers().0,
            rules_in_their_regions,
        }
    } else {
        // A different literal or input is never shared.
        Fused::Separate {
            distinct: emits.len(),
            rules_in_their_regions,
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
