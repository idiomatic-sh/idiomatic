//! Frontier and callback delivery share one atomic failure and one dependency contract in both orders.
use super::super::super::fixtures::*;
use crate::boundary_fixture::fixtures::{authored_request, corpus, ran};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::{
    EmittedRoot, FileEnd, RootOutcome, RuleAfter,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId};
use ryo_values::domain::plans::types::{Operation, PlanStep};
#[test]
fn frontier_and_callback_share_the_same_atomic_semantic_failure() {
    {
        let (engine, _) = engine();
        let mut authored = authored_request(1);
        crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps[0].operation =
            Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::DynamicRegex),
                kept: vec![],
            };
        crate::boundary_fixture::fixtures::plans(&mut authored)[0].frontier =
            vec![Input::Step(StepId(0))];
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let (elements, mut symbols) = tree(FileId(0), b"ab(");
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut rules = run(
            &request,
            (FileId(0), b"ab("),
            &elements,
            &intents,
            &mut symbols,
        );
        let rule = ran(&mut rules, 0);
        assert!(
            matches!(&rule.emit, EmittedRoot::Delivered(RootOutcome::Failed(error)) if *error == regex_error("(")),
            "failed evaluation publishes no prefix"
        );
        assert!(
            matches!(rule.after, RuleAfter::Unreached),
            "a prefix followed by failure is never a complete frontier"
        );
        assert!(matches!(rules.end, FileEnd::Unfinished));
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    // Each rule runs on its own, the other undemanded: the forwarded and the collected diagnostic answer one error.
    for intents in [
        [
            FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            },
            FileRuleIntent::Undemanded,
        ],
        [
            FileRuleIntent::Undemanded,
            FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            },
        ],
    ] {
        let (elements, mut symbols) = tree(FileId(0), b"abc");
        let mut rules = run(
            &request,
            (FileId(0), b"abc"),
            &elements,
            &intents,
            &mut symbols,
        );
        let rule = rules
            .rules
            .first()
            .map(|each| each.rule.0)
            .expect("one rule ran");
        assert_eq!(
            failed(&mut rules, rule),
            Some(regex_error("[")),
            "forwarded empty Join has no diagnostic rows"
        );
    }
    request.finish();
}
fn conditional_join_request(count: usize) -> PrepareRequest {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let mut authored = authored_request(count);
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![
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
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(2);
    for index in 1..count {
        crate::boundary_fixture::fixtures::plans(&mut authored)[index] =
            crate::boundary_fixture::fixtures::plans(&mut authored)[0].clone();
    }
    authored
}
#[test]
fn callback_and_frontier_use_one_dependency_contract_in_both_orders() {
    let (engine, _) = engine();
    let mut authored = conditional_join_request(1);
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].frontier =
        vec![Input::Step(StepId(2))];
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }];
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &intents,
        &mut symbols,
    );
    let rule = ran(&mut rules, 0);
    assert!(
        matches!(&rule.emit, EmittedRoot::Delivered(RootOutcome::Failed(error)) if *error == regex_error("[")),
        "empty forwarded Join cannot emit a row"
    );
    assert!(
        matches!(rule.after, RuleAfter::Unreached),
        "frontier and diagnostic must report the same failing RHS: no frontier is captured past it"
    );
    request.finish();
}
