//! Earlier semantic errors are never hidden by later ones and are terminal for every retry.
use super::super::super::fixtures::*;
use super::super::super::no_catalog;
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}
#[test]
fn an_earlier_false_kind_test_still_short_circuits_the_later_invalid_regex() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::AbsentKindBeforeRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}
#[test]
fn a_fused_child_error_precedes_a_later_head_error_and_is_terminal() {
    let (engine, _semantics) = engine();
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
                from: Input::Step(StepId(0)),
                comparisons: vec![compared(Test::Matches, 2, text("["))],
                kept: vec![],
            },
        },
    ];
    crate::boundary_fixture::plans(&mut authored)[0].emit.step = StepId(1);
    let program = engine
        .prepare(authored)
        .expect("prepare actual fused head and child");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
    failed(
        file.rule(RuleIndex(0), demand(), &mut symbols).unwrap(),
        child_error,
    );
    file.finish().expect("finish failed continuation owner");
    request.finish();
}
#[test]
fn semantic_failure_precedes_delivery_and_is_terminal_for_every_retry() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::DynamicRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
    let mut delivered = vec![];
    let retried = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        delivered.push(row.to_vec());
        Ok(())
    });
    assert!(matches!(retried,Err(ref error) if *error==regex_error("(")));
    assert!(delivered.is_empty());
    assert_eq!(
        semantics.selects(),
        0,
        "cached native rows never rerun ordinary selection"
    );
    let terminal = file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
        panic!("terminal semantic failure must not invoke an external sink again")
    });
    assert!(matches!(terminal,Err(ref error) if *error==regex_error("(")));
    file.finish().unwrap();
    request.finish();
}
/// S13: an operator behind the port that keeps calling its lent driver after a refusal does not replace the first refusal: the
/// run reports the first engine error of the evaluation.
#[derive(Debug)]
struct CallsOnAfterARefusal;
impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for CallsOnAfterARefusal {
    fn operators(
        &self,
        mut call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase as _;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        if let OperatorCall::Evaluate {
            operation: Operation::Select { .. },
            driver,
            ..
        } = &mut call
        {
            // A row-mode evaluation lends no population: refused `NotRun`, the first error.
            let _population =
                driver.population(ryo_values::domain::plan_parts::types::InputNumber(99));
            // A Select is no RenderSyntax: refused `WrongOperation`, the later error.
            let _render = driver.render(&[]);
            return Err(ryo_operators::domain::contract::types::PortableOperatorFault::Interrupted);
        }
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
            std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
        )
        .operators(call)
    }
}
#[test]
fn the_first_engine_error_of_an_evaluation_is_the_one_reported() {
    let engine = ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl::new(
        std::sync::Arc::new(CallsOnAfterARefusal),
        std::sync::Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
        std::sync::Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    // A Select of the syntax tree is answered by the shared walk; a Select over its step is evaluated by the operators, in row
    // mode.
    let mut authored = authored_request(1);
    crate::boundary_fixture::plans(&mut authored)[0].steps = vec![
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
    crate::boundary_fixture::plans(&mut authored)[0].emit.step = StepId(1);
    let program = engine
        .prepare(authored)
        .expect("prepare a select over a select");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"abc",
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
        .expect("open the file");
    failed(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .expect("the rule reports its outcome"),
        OperatingError::NotRun,
    );
    let _finished = file.finish();
    request.finish();
}
