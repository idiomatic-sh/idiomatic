//! Actual public file-owner readiness bounds and terminal failure replay.
use super::super::{fixtures::no_catalog, fixtures::*};
use super::fixtures::{readiness_chain, readiness_expected, readiness_hosted, readiness_intent};
use crate::boundary_fixture::fixtures::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{HostedNumber, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::{RowCount, Table},
};
#[test]
fn diamond_deep_chains_and_duplicate_inputs_wake_only_actual_waiters() {
    for empty in [false, true] {
        exercise_readiness_chain(empty);
    }
}
fn exercise_readiness_chain(empty: bool) {
    let (engine, _) = engine();
    let mut authored = authored_request(3);
    let steps = readiness_chain();
    for (index, plan) in crate::boundary_fixture::fixtures::plans(&mut authored)
        .iter_mut()
        .enumerate()
    {
        plan.steps = steps.clone();
        let last = Input::Step(StepId(u32::try_from(steps.len() - 1).unwrap()));
        let first = Input::Step(StepId(0));
        let (left, right) = if index == 1 {
            (first, last)
        } else {
            (last, first)
        };
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union { left, right },
        });
        plan.emit.step = StepId(u32::try_from(plan.steps.len() - 1).unwrap());
    }
    authored.hosted = vec![readiness_hosted()];
    if empty {
        authored.hosted[0].rows.clear();
    }
    let expected = if empty {
        Table {
            kinds: vec![CellKind::Number],
            count: RowCount(0),
            cells: vec![],
        }
    } else {
        readiness_expected()
    };
    let prepared = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [readiness_intent(), readiness_intent(), readiness_intent()];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap()),
        expected.clone()
    );
    assert_eq!(
        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
        expected.clone()
    );
    assert_eq!(
        complete(file.rule(RuleIndex(2), demand(), &mut symbols).unwrap()),
        expected.clone()
    );
    file.finish().unwrap();
    request.finish();
}
#[test]
fn failed_producers_wake_their_roots_without_erasing_independent_output() {
    let (engine, _) = engine();
    let mut authored = authored_request(3);
    let mut failing = readiness_chain();
    failing[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![],
    };
    for (index, plan) in crate::boundary_fixture::fixtures::plans(&mut authored)[..2]
        .iter_mut()
        .enumerate()
    {
        plan.steps = failing.clone();
        let last = Input::Step(StepId(u32::try_from(failing.len() - 1).unwrap()));
        let first = Input::Step(StepId(0));
        let (left, right) = if index == 0 {
            (first, last)
        } else {
            (last, first)
        };
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union { left, right },
        });
        plan.emit.step = StepId(u32::try_from(plan.steps.len() - 1).unwrap());
    }
    crate::boundary_fixture::fixtures::plans(&mut authored)[2].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
    }];
    authored.hosted = vec![readiness_hosted()];
    let prepared = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let calls = std::cell::RefCell::new(Vec::new());
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        calls.borrow_mut().push(relation);
        Err::<SourceRows<'_>, _>(OperatingError::NotRead)
    };
    let intents = [readiness_intent(), readiness_intent(), readiness_intent()];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &read),
            &mut symbols,
        )
        .unwrap();
    for index in [0, 1] {
        let outcome = file.rule(RuleIndex(index), demand(), &mut symbols).unwrap();
        failed(outcome, OperatingError::NotRead);
    }
    assert_eq!(
        *calls.borrow(),
        [Relation::Function],
        "the shared pure source is borrowed once; waiting ancestors do not retry it"
    );
    let again = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
    failed(again, OperatingError::NotRead);
    assert_eq!(*calls.borrow(), [Relation::Function]);
    let mut expected = readiness_expected();
    expected.count = RowCount(4);
    expected.cells = [3, 1, 3, 2]
        .into_iter()
        .map(|n| Value::Number(Integer(n)))
        .collect();
    assert_eq!(
        complete(file.rule(RuleIndex(2), demand(), &mut symbols).unwrap()),
        expected
    );
    file.finish().unwrap();
    request.finish();
}
