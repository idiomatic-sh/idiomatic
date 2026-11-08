//! Frontier and callback delivery share one atomic failure and one dependency contract in both orders.
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use super::super::super::fixtures::*;
use super::super::super::no_catalog;
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId};
use ryo_values::domain::plans::types::{Operation, PlanStep};
#[test]
fn frontier_and_callback_share_the_same_atomic_semantic_failure() {
    use ryo_relations::domain::relations::types::{Words, WorkerNumber};
    for capture_first in [true, false] {
        let (engine, _) = engine();
        let mut authored = authored_request(1);
        crate::boundary_fixture::plans(&mut authored)[0].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: selection(Selector::DynamicRegex),
            kept: vec![],
        };
        crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::Step(StepId(0))];
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
        let (elements, mut symbols) = tree(FileId(0), b"ab(");
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Forwarded,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut file = request
            .open_file(
                input(FileId(0), b"ab(", &elements, &intents, &no_catalog),
                &mut symbols,
            )
            .unwrap();
        for capture_now in [capture_first, !capture_first] {
            if capture_now {
                let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
                let captured = file
                    .capture_frontiers(
                        RuleIndex(0),
                        ryo_values::domain::stored_facts::types::Bucket(0),
                        &mut symbols,
                        CaptureContext {
                            worker: WorkerNumber(0),
                            arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                            places: &mut places,
                            words: &mut words,
                        },
                    )
                    .unwrap();
                assert_eq!(captured.len(), 1);
                assert!(
                    matches!(&captured[0].outcome, Err(error) if *error == regex_error("(")),
                    "a prefix followed by failure is never a complete frontier"
                );
            } else {
                let mut delivered = vec![];
                let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    delivered.extend_from_slice(row);
                    Ok(())
                });
                assert!(matches!(result, Err(error) if error == regex_error("(")));
                assert!(
                    delivered.is_empty(),
                    "failed evaluation publishes no prefix"
                );
            }
        }
        counts(&file, 1, 4);
        assert_eq!(file.walk_counts().predicate_rows.0, 2);
        file.finish().unwrap();
        request.finish();
    }
}
#[test]
fn identical_join_diagnostics_have_equal_errors_for_both_delivery_orders() {
    let (engine, _) = engine();
    let authored = conditional_join_request(2);
    let program = engine
        .prepare(authored)
        .expect("identical authored Join plans");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Forwarded,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        },
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        },
    ];
    for invocation_order in [[RuleIndex(0), RuleIndex(1)], [RuleIndex(1), RuleIndex(0)]] {
        let (elements, mut symbols) = tree(FileId(0), b"abc");
        let mut file = request
            .open_file(
                input(FileId(0), b"abc", &elements, &intents, &no_catalog),
                &mut symbols,
            )
            .expect("fresh file owner with opposite diagnostic modes");
        for rule in invocation_order {
            if rule == RuleIndex(0) {
                let mut rows = Vec::new();
                let emission = file.emit_rows(rule, &mut symbols, &mut |row| {
                    rows.push(row.to_vec());
                    Ok(())
                });
                assert!(matches!(emission, Err(ref error) if *error == regex_error("[")));
                assert!(
                    rows.is_empty(),
                    "forwarded empty Join has no diagnostic rows"
                );
            } else {
                failed(
                    file.rule(rule, demand(), &mut symbols)
                        .expect("collected diagnostic outcome"),
                    regex_error("["),
                );
            }
        }
        file.finish().expect("release both diagnostic terminals");
    }
    request.finish();
}
fn conditional_join_request(count: usize) -> PrepareRequest {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let mut authored = authored_request(count);
    crate::boundary_fixture::plans(&mut authored)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::Kind("absent-kind")),
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::BadRegex),
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    crate::boundary_fixture::plans(&mut authored)[0].emit.step = StepId(2);
    for index in 1..count {
        crate::boundary_fixture::plans(&mut authored)[index] = crate::boundary_fixture::plans(&mut authored)[0].clone();
    }
    authored
}
#[test]
fn callback_and_frontier_use_one_dependency_contract_in_both_orders() {
    let (engine, _) = engine();
    use ryo_relations::domain::relations::types::{Words, WorkerNumber};
    let mut authored = conditional_join_request(1);
    crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::Step(StepId(2))];
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Forwarded,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }];
    for capture_first in [true, false] {
        let (elements, mut symbols) = tree(FileId(0), b"abc");
        let mut file = request
            .open_file(
                input(FileId(0), b"abc", &elements, &intents, &no_catalog),
                &mut symbols,
            )
            .unwrap();
        for capture_now in [capture_first, !capture_first] {
            if capture_now {
                let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
                let captured = file
                    .capture_frontiers(
                        RuleIndex(0),
                        ryo_values::domain::stored_facts::types::Bucket(0),
                        &mut symbols,
                        CaptureContext {
                            worker: WorkerNumber(0),
                            arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                            places: &mut places,
                            words: &mut words,
                        },
                    )
                    .unwrap();
                assert_eq!(captured.len(), 1);
                assert!(
                    matches!(&captured[0].outcome, Err(error) if *error == regex_error("[")),
                    "frontier and diagnostic must report the same failing RHS"
                );
            } else {
                let outcome = file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
                    panic!("empty forwarded Join cannot emit a row")
                });
                assert!(matches!(outcome, Err(ref error) if *error == regex_error("[")));
            }
        }
        file.finish().unwrap();
    }
    request.finish();
}
