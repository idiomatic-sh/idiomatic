//! Actual public file-owner readiness bounds and terminal failure.
use super::super::fixtures::*;
use super::fixtures::{readiness_chain, readiness_expected, readiness_hosted};
use crate::boundary_fixture::fixtures::{FileRun, authored_request, corpus, run_file, source};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::{FileEnd, OperatingError};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, HostedNumber, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::{
        errors::RowsError,
        types::{RowCount, Table},
    },
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
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [COLLECTED, COLLECTED, COLLECTED];
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &intents,
        &mut symbols,
    );
    assert_eq!(complete(&mut rules, 0), expected.clone());
    assert_eq!(complete(&mut rules, 1), expected.clone());
    assert_eq!(complete(&mut rules, 2), expected.clone());
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}
#[test]
fn failed_producers_wake_their_roots_without_erasing_independent_output() {
    let (engine, _) = engine();
    let mut authored = authored_request(3);
    let mut failing = readiness_chain();
    // The shared pure source is read, and the producer over it fails on a column it does not have.
    failing[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(99)],
    };
    // The independent rule runs first; the failing producers after it, the first of which stops the file.
    for (index, plan) in crate::boundary_fixture::fixtures::plans(&mut authored)[1..]
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
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
    }];
    authored.hosted = vec![readiness_hosted()];
    let prepared = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let functions = source();
    let read = TableFacts::new(&[(Relation::Function, &functions)]);
    let intents = [COLLECTED, COLLECTED, COLLECTED];
    let mut rules = run_file(
        &request,
        FileRun {
            file: FileId(0),
            source: b"abc",
            ..FileRun::of(&intents, &elements)
        },
        &mut symbols,
        &read,
    );
    let mut expected = readiness_expected();
    expected.count = RowCount(4);
    expected.cells = [3, 1, 3, 2]
        .into_iter()
        .map(|n| Value::Number(Integer(n)))
        .collect();
    assert_eq!(complete(&mut rules, 0), expected);
    assert_eq!(
        failed(&mut rules, 1),
        Some(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
    );
    assert!(
        read.history()
            .iter()
            .all(|relation| *relation == Relation::Function),
        "the failing producers read the shared pure source alone"
    );
    assert!(matches!(rules.end, FileEnd::Unfinished));
    request.finish();
}
