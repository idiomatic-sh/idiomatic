//! Dormant declarations cannot allocate cohort scratch or retained result slots.
use super::super::fixtures::*;
use crate::boundary_fixture::fixtures::{authored_request, corpus};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
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
        let (plans, bindings) = crate::boundary_fixture::fixtures::authored_parts(&mut authored);
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
                        Operand::Literal(QueryLiteral::Number(Integer(
                            i64::try_from(index).expect("a declaration number fits in i64"),
                        ))),
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
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let mut intents = vec![FileRuleIntent::Undemanded; declarations];
        intents[0] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        // Reuse the immutable request across fresh file generations.
        for _generation in 0..2 {
            let (elements, mut symbols) = tree(FileId(0), b"abc");
            let mut rules = run(
                &request,
                (FileId(0), b"abc"),
                &elements,
                &intents,
                &mut symbols,
            );
            assert_eq!(
                complete(&mut rules, 0),
                expected(FileId(0), &elements, &[1, 3])
            );
            assert_eq!(semantics.selects(), 0);
            assert!(matches!(rules.end, FileEnd::Finished(_)));
        }
        request.finish();
    }
}

#[test]
fn a_failed_dependency_does_not_prepare_a_later_class_of_the_same_rule() {
    let (engine, semantics) = engine();
    let mut authored = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![
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
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(2);
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }];
    let mut rules = run(
        &request,
        (FileId(0), b"[bc"),
        &elements,
        &intents,
        &mut symbols,
    );
    assert_eq!(failed(&mut rules, 0), Some(regex_error("[")));
    assert_eq!(semantics.selects(), 0);
    request.finish();
}

#[test]
fn later_cohorts_add_only_the_new_class_and_aliases_replay_both_results() {
    let (engine, semantics) = engine();
    let mut authored = authored_request(2);
    for plan in crate::boundary_fixture::fixtures::plans(&mut authored) {
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }; 2];
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &intents,
        &mut symbols,
    );
    for rule in [1, 0] {
        assert_eq!(
            complete(&mut rules, rule),
            expected(FileId(0), &elements, &[1, 3, 2])
        );
    }
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}

#[test]
fn full_rule_count_shares_source_decoding_and_ordered_predicate_prefixes() {
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    const RULES: usize = 413;
    let (engine, semantics) = engine();
    let mut authored = authored_request(RULES);
    let (plans, bindings) = crate::boundary_fixture::fixtures::authored_parts(&mut authored);
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = vec![
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        RULES
    ];
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &intents,
        &mut symbols,
    );
    for index in (0..RULES).rev() {
        let actual = complete(&mut rules, index);
        assert_eq!(actual.count.0, 2);
        assert_eq!(actual.kinds.len(), 1);
    }
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Finished(_)));
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
    let (plans, bindings) = crate::boundary_fixture::fixtures::authored_parts(&mut authored);
    for (index, plan) in plans.iter_mut().enumerate() {
        plan.parameters.push(PlanParameter {
            name: ParameterName("pattern".into()),
            kind: ParameterType::Text,
        });
        bindings[index] = vec![ParameterBinding::Text(
            ryo_values::domain::plan_parts::types::LiteralText(
                if index == 2 { "[" } else { "a" }.into(),
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
            text(if index == 0 { "different" } else { "absent" }),
        ));
        plan.steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons,
            kept: vec![],
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }; 3];
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &intents,
        &mut symbols,
    );
    for rule in [0, 1] {
        assert_eq!(
            complete(&mut rules, rule),
            expected(FileId(0), &elements, &[1])
        );
    }
    // The rule whose bound regex is invalid answers its own first error; the file stops at it.
    assert_eq!(failed(&mut rules, 2), Some(regex_error("[")));
    // Kind tests share across all classes, while only the two equal bound regexes share.
    assert_eq!(semantics.selects(), 0);
    request.finish();
}
