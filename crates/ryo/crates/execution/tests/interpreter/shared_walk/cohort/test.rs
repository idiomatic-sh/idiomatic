//! Dormant declarations cannot allocate cohort scratch or retained result slots.
use crate::shared_walk::{fixtures::*, no_catalog};
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_relations::domain::relations::types::{FileId, Integer, Relation};
use ryo_values::domain::{
    plan_parts::types::{Input, Operand, QueryLiteral, StepId, StepScope, Test},
    plans::types::{Operation, PlanStep},
};

#[test]
fn only_admitted_classes_allocate_scratch_and_result_slots() {
    for declarations in [1, 257] {
        let (engine, semantics) = engine();
        let mut authored = authored_request(declarations);
        let (plans, bindings) = crate::boundary_fixture::authored_parts(&mut authored);
        for (index, plan) in plans.iter_mut().enumerate() {
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
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
            assert_eq!(file.fixture_walk_counts().scratch_classes.0, 0);
            assert_eq!(file.fixture_walk_counts().result_slots.0, 0);
            let result = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            assert_eq!(complete(result), expected(FileId(0), &elements, &[1, 3]));
            let measured = file.fixture_walk_counts();
            assert_eq!(measured.scratch_classes.0, 1);
            assert_eq!(measured.result_slots.0, 1);
            assert_eq!(measured.passes.0, 1);
            assert_eq!(measured.source_rows.0, 4);
            assert_eq!(semantics.selects(), 0);
            let replay = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            assert_eq!(complete(replay), expected(FileId(0), &elements, &[1, 3]));
            assert_eq!(file.fixture_walk_counts().scratch_classes, measured.scratch_classes);
            assert_eq!(file.fixture_walk_counts().result_slots, measured.result_slots);
            file.finish().unwrap();
        }
        request.finish();
    }
}

#[test]
fn a_failed_dependency_does_not_prepare_a_later_class_of_the_same_rule() {
    let (engine, semantics) = engine();
    let mut authored = authored_request(1);
    crate::boundary_fixture::plans(&mut authored)[0].steps = vec![
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
    crate::boundary_fixture::plans(&mut authored)[0].emit.step = StepId(2);
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
        let actual = file.fixture_walk_counts();
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
    for plan in crate::boundary_fixture::plans(&mut authored) {
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
        let actual = file.fixture_walk_counts();
        assert_eq!(actual.scratch_classes.0, 2);
        assert_eq!(actual.result_slots.0, 2);
        assert_eq!(actual.predicate_rows.0, 3);
    }
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}

#[test]
fn full_rule_count_shares_source_decoding_and_ordered_predicate_prefixes() {
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    const RULES: usize = 413;
    let (engine, semantics) = engine();
    let mut authored = authored_request(RULES);
    let (plans, bindings) = crate::boundary_fixture::authored_parts(&mut authored);
    for (index, plan) in plans.iter_mut().enumerate() {
        let mut comparisons = selection(Selector::Kind("word"));
        comparisons.push(compared(
            Test::NotEqual,
            2,
            text(&format!("absent-{index}")),
        ));
        plan.steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons,
            kept: vec![ColumnIndex(2)],
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = vec![
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        };
        RULES
    ];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(file.fixture_walk_counts().predicate_tests.0, 0);
    for index in (0..RULES).rev() {
        let actual = complete(file.rule(RuleIndex(index), demand(), &mut symbols).unwrap());
        assert_eq!(actual.count.0, 2);
        assert_eq!(actual.kinds.len(), 1);
    }
    let actual = file.fixture_walk_counts();
    assert_eq!(actual.passes.0, 1);
    assert_eq!(actual.source_rows.0, 4);
    assert_eq!(actual.decoded_rows.0, 2);
    assert_eq!(actual.predicate_rows.0, 2 * RULES as u64);
    assert_eq!(actual.predicate_tests.0, 2 * (RULES as u64 + 1));
    assert_eq!(actual.predicate_reuses.0, 2 * (RULES as u64 - 1));
    assert_eq!(actual.result_slots.0, RULES as u64);
    assert_eq!(semantics.selects(), 0);
    let replay = complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap());
    assert_eq!(replay.count.0, 2);
    assert_eq!(file.fixture_walk_counts(), actual);
    file.finish().unwrap();
    request.finish();
}

#[test]
fn shared_prefixes_preserve_distinct_parameter_bindings_and_first_errors() {
    use ryo_values::domain::{
        plan_parts::types::{ParameterId, ParameterName},
        query_bindings::types::{ParameterBinding, ParameterType, PlanParameter},
    };
    let (engine, semantics) = engine();
    let mut authored = authored_request(3);
    let (plans, bindings) = crate::boundary_fixture::authored_parts(&mut authored);
    for (index, plan) in plans.iter_mut().enumerate() {
        plan.parameters.push(PlanParameter {
            name: ParameterName("pattern".into()),
            kind: ParameterType::Text,
        });
        bindings[index] = vec![ParameterBinding::Text(
            ryo_values::domain::plan_parts::types::LiteralText(
                if index == 0 { "[" } else { "a" }.into(),
            ),
        )];
        let mut comparisons = selection(Selector::Kind("word"));
        comparisons.push(compared(
            Test::Matches,
            2,
            Operand::Parameter(ParameterId(0)),
        ));
        comparisons.push(compared(
            Test::NotEqual,
            2,
            text(if index == 2 { "different" } else { "absent" }),
        ));
        plan.steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons,
            kept: vec![],
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }; 3];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    failed(
        file.rule(RuleIndex(0), demand(), &mut symbols).unwrap(),
        regex_error("["),
    );
    for rule in [RuleIndex(2), RuleIndex(1)] {
        assert_eq!(
            complete(file.rule(rule, demand(), &mut symbols).unwrap()),
            expected(FileId(0), &elements, &[1])
        );
    }
    let actual = file.fixture_walk_counts();
    assert_eq!(actual.passes.0, 1);
    assert_eq!(actual.decoded_rows.0, 2);
    // Kind tests share across all classes, while only the two equal bound regexes share.
    assert_eq!(actual.predicate_tests.0, 7);
    assert_eq!(actual.predicate_reuses.0, 5);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}
