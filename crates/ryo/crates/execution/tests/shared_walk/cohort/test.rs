//! Dormant declarations cannot allocate cohort scratch or retained result slots.
use super::super::{fixtures::*, no_catalog};
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
};
use ryo_relations::domain::relations::types::{FileId, Integer, Relation};
use ryo_values::domain::{
    plan_parts::types::{Input, Operand, QueryLiteral, StepId, Test},
    plans::types::{Operation, PlanStep, StepScope},
};

#[test]
fn only_admitted_classes_allocate_scratch_and_result_slots() {
    for declarations in [1, 257] {
        let (engine, semantics) = engine();
        let mut authored = authored_request(declarations);
        for (index, plan) in authored.plans.iter_mut().enumerate() {
            let (relation, comparisons) = if index == 0 {
                (Relation::SyntaxTree, selection(Selector::Kind("word")))
            } else if index % 2 == 0 {
                (
                    Relation::SyntaxTree,
                    vec![compared(Test::Equal, 1, text(&format!("dormant-{index}")))],
                )
            } else {
                (
                    Relation::LanguageTarget,
                    vec![compared(
                        Test::Equal,
                        1,
                        Operand::Literal(QueryLiteral::Number(Integer(index as i64))),
                    )],
                )
            };
            plan.steps[0].operation = Operation::Select {
                from: Input::Relation(relation),
                comparisons,
                kept: vec![],
            };
        }
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let mut intents = vec![FileRuleIntent::Undemanded; declarations];
        intents[0] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        };
        // Reuse the immutable request across fresh file generations.
        for _generation in 0..2 {
            let (elements, mut symbols) = tree(FileId(0), b"abc");
            let mut file = request
                .open_file(
                    input(FileId(0), b"abc", &elements, &intents, &no_catalog),
                    &mut symbols,
                )
                .unwrap();
            assert_eq!(file.walk_counts().scratch_classes.0, 0);
            assert_eq!(file.walk_counts().result_slots.0, 0);
            let result = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            assert_eq!(complete(result), expected(FileId(0), &elements, &[1, 3]));
            let measured = file.walk_counts();
            assert_eq!(measured.scratch_classes.0, 1);
            assert_eq!(measured.result_slots.0, 1);
            assert_eq!(measured.passes.0, 1);
            assert_eq!(measured.source_rows.0, 4);
            assert_eq!(semantics.selects(), 0);
            let replay = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            assert_eq!(complete(replay), expected(FileId(0), &elements, &[1, 3]));
            assert_eq!(file.walk_counts().scratch_classes, measured.scratch_classes);
            assert_eq!(file.walk_counts().result_slots, measured.result_slots);
            file.finish().unwrap();
        }
        request.finish();
    }
}

#[test]
fn a_failed_dependency_does_not_prepare_a_later_class_of_the_same_rule() {
    let (engine, semantics) = engine();
    let mut authored = authored_request(1);
    authored.plans[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::DynamicRegex),
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::Kind("other")),
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
            },
        },
    ];
    authored.plans[0].emit.step = StepId(2);
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    let mut file = request
        .open_file(
            input(FileId(0), b"[bc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    for _delivery in 0..2 {
        failed(
            file.rule(RuleIndex(0), demand(), &mut symbols).unwrap(),
            regex_error("["),
        );
        let actual = file.walk_counts();
        assert_eq!(actual.scratch_classes.0, 1);
        assert_eq!(actual.result_slots.0, 1);
        assert_eq!(actual.passes.0, 1);
        assert_eq!(actual.predicate_rows.0, 1);
    }
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}

#[test]
fn later_cohorts_add_only_the_new_class_and_aliases_replay_both_results() {
    let (engine, semantics) = engine();
    let mut authored = authored_request(2);
    for plan in &mut authored.plans {
        plan.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::SyntaxTree),
                    comparisons: selection(Selector::Kind("word")),
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::SyntaxTree),
                    comparisons: selection(Selector::Kind("other")),
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Union {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                },
            },
        ];
        plan.emit.step = StepId(2);
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }; 2];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    for rule in [RuleIndex(1), RuleIndex(0), RuleIndex(1)] {
        let result = file.rule(rule, demand(), &mut symbols).unwrap();
        assert_eq!(complete(result), expected(FileId(0), &elements, &[1, 3, 2]));
        let actual = file.walk_counts();
        assert_eq!(actual.scratch_classes.0, 2);
        assert_eq!(actual.result_slots.0, 2);
        assert_eq!(actual.predicate_rows.0, 3);
    }
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}
