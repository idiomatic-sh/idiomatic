//! Earlier semantic errors are never hidden by later ones and are terminal for every retry.
use super::super::super::fixtures::*;
use crate::boundary_fixture::fixtures::{authored_request, corpus};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution_runtime::outbound::operators::fixtures::order_error_order::CallsOnAfterARefusal;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, Operand, StepId, Test};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::types::{RowCount, Table};
#[test]
fn an_earlier_dynamic_regex_error_is_not_hidden_by_a_later_false_kind_test() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::RegexBeforeAbsentKind]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let mut rules = run(
        &request,
        (FileId(0), b"[bc"),
        &elements,
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        &mut symbols,
    );
    assert_eq!(failed(&mut rules, 0), Some(regex_error("[")));
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Unfinished));
    request.finish();
}
#[test]
fn an_earlier_false_kind_test_still_short_circuits_the_later_invalid_regex() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::AbsentKindBeforeRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let mut rules = run(
        &request,
        (FileId(0), b"[bc"),
        &elements,
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        &mut symbols,
    );
    assert_eq!(
        complete(&mut rules, 0),
        Table {
            kinds: kinds(Relation::SyntaxTree).to_vec(),
            count: RowCount(0),
            cells: vec![]
        }
    );
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}
#[test]
fn a_fused_child_error_precedes_a_later_head_error_and_is_terminal() {
    let (engine, _semantics) = engine();
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
                from: Input::Step(StepId(0)),
                comparisons: vec![compared(Test::Matches, 2, text("["))],
                kept: vec![],
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(1);
    let program = engine
        .prepare(authored)
        .expect("prepare actual fused head and child");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"ab(");
    let mut rules = run(
        &request,
        (FileId(0), b"ab("),
        &elements,
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        &mut symbols,
    );
    let child_error = regex_error("[");
    // Regex compilation intentionally exposes the same generic reason for
    // these malformed patterns. The independent continuation count below proves
    // ordering instead of attributing the failure from indistinguishable text.
    assert_eq!(failed(&mut rules, 0), Some(child_error));
    assert!(matches!(rules.end, FileEnd::Unfinished));
    request.finish();
}
#[test]
fn semantic_failure_precedes_delivery_and_is_terminal_for_every_retry() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::DynamicRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"ab(");
    let mut rules = run(
        &request,
        (FileId(0), b"ab("),
        &elements,
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        &mut symbols,
    );
    assert_eq!(
        failed(&mut rules, 0),
        Some(regex_error("(")),
        "semantic failure must precede all delivery"
    );
    assert_eq!(
        semantics.selects(),
        0,
        "cached native rows never rerun ordinary selection"
    );
    assert!(matches!(rules.end, FileEnd::Unfinished));
    request.finish();
}
#[test]
fn the_first_engine_error_of_an_evaluation_is_the_one_reported() {
    let engine = ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl::new(
        std::sync::Arc::new(CallsOnAfterARefusal),
        std::sync::Arc::new(ryo_execution::inbound::use_cases::execution::fixtures::derived())
            .into(),
        std::sync::Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    // A Select of the syntax tree is answered by the shared walk; a Select over its step is evaluated by the operators, in row
    // mode.
    let mut authored = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![
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
                from: Input::Step(StepId(0)),
                comparisons: vec![compared(Test::Equal, 0, Operand::Column(ColumnIndex(0)))],
                kept: vec![ColumnIndex(0)],
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(1);
    let program = engine
        .prepare(authored)
        .expect("prepare a select over a select");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        &mut symbols,
    );
    assert_eq!(failed(&mut rules, 0), Some(OperatingError::NotRun));
    request.finish();
}
