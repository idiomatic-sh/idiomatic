//! Earlier semantic errors are never hidden by later ones and are terminal for every retry.
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use super::super::super::fixtures::*;
use super::super::super::no_catalog;
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{Input, StepId, Test};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::types::{RowCount, Table};
#[test]
fn an_earlier_dynamic_regex_error_is_not_hidden_by_a_later_false_kind_test() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::RegexBeforeAbsentKind]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"[bc",
                &elements,
                &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    failed(
        file.rule(RuleIndex(0), demand(), &mut symbols).unwrap(),
        regex_error("["),
    );
    counts(&file, 1, 4);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}
#[test]
fn an_earlier_false_kind_test_still_short_circuits_the_later_invalid_regex() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::AbsentKindBeforeRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"[bc",
                &elements,
                &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap()),
        Table {
            kinds: kinds(Relation::SyntaxTree).to_vec(),
            count: RowCount(0),
            cells: vec![]
        }
    );
    counts(&file, 0, 0);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}
#[test]
fn a_fused_child_error_precedes_a_later_head_error_and_is_terminal() {
    let (engine, _semantics) = engine();
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
                from: Input::Step(StepId(0)),
                comparisons: vec![compared(Test::Matches, 2, text("["))],
                kept: vec![],
            },
        },
    ];
    authored.plans[0].emit.step = StepId(1);
    let program = engine
        .prepare(authored)
        .expect("prepare actual fused head and child");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"ab(");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"ab(",
                &elements,
                &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    let child_error = regex_error("[");
    // Regex compilation intentionally exposes the same generic reason for
    // these malformed patterns. The independent continuation count below proves
    // ordering instead of attributing the failure from indistinguishable text.
    failed(
        file.rule(RuleIndex(0), demand(), &mut symbols).unwrap(),
        child_error.clone(),
    );
    counts(&file, 1, 4);
    let work: Vec<_> = file
        .counts(RuleIndex(0))
        .unwrap()
        .fed
        .iter()
        .map(|fed| fed.0)
        .collect();
    assert_eq!(
        work,
        [[2, 0, 0], [1, 0, 0]],
        "the shared head visits both word rows, but its accepted first row must reach the child before the cached later head error; returning the head error first leaves child work at zero"
    );
    failed(
        file.rule(RuleIndex(0), demand(), &mut symbols).unwrap(),
        child_error,
    );
    assert_eq!(
        file.counts(RuleIndex(0))
            .unwrap()
            .fed
            .iter()
            .map(|fed| fed.0)
            .collect::<Vec<_>>(),
        work,
        "semantic child failure is terminal; the continuation is not retried"
    );
    counts(&file, 1, 4);
    file.finish().expect("finish failed continuation owner");
    request.finish();
}
#[test]
fn semantic_failure_precedes_delivery_and_is_terminal_for_every_retry() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::DynamicRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"ab(");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"ab(",
                &elements,
                &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Forwarded,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    let mut attempts = 0;
    let refused = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        attempts += 1;
        let _ = row;
        Err(OperatingError::WrongBinding)
    });
    assert!(matches!(refused, Err(ref error) if *error == regex_error("(")));
    assert_eq!(attempts, 0, "semantic failure must precede all delivery");
    counts(&file, 1, 4);
    let mut delivered = vec![];
    let retried = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        delivered.push(row.to_vec());
        Ok(())
    });
    assert!(matches!(retried,Err(ref error) if *error==regex_error("(")));
    assert!(delivered.is_empty());
    counts(&file, 1, 4);
    assert_eq!(
        semantics.selects(),
        0,
        "cached native rows never rerun ordinary selection"
    );
    let work: Vec<_> = file
        .counts(RuleIndex(0))
        .unwrap()
        .fed
        .iter()
        .map(|fed| fed.0)
        .collect();
    assert!(work.iter().any(|fed| *fed != [0; 3]));
    let terminal = file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
        panic!("terminal semantic failure must not invoke an external sink again")
    });
    assert!(matches!(terminal,Err(ref error) if *error==regex_error("(")));
    assert_eq!(
        file.counts(RuleIndex(0))
            .unwrap()
            .fed
            .iter()
            .map(|fed| fed.0)
            .collect::<Vec<_>>(),
        work
    );
    counts(&file, 1, 4);
    file.finish().unwrap();
    request.finish();
}
