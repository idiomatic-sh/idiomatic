//! Shared cached prefixes preserve authored error order and the existing retry protocol.
use super::{fixture::*, no_catalog};
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Relation},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, Test},
    plans::types::{Operation, PlanStep, StepScope},
    rows::types::{RowCount, Table},
};

#[test]
fn an_earlier_dynamic_regex_error_is_not_hidden_by_a_later_false_kind_test() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::RegexBeforeAbsentKind]);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let mut file = request
        .open_file(
            input(FileId(0), b"[bc", &elements, &[true], &no_catalog),
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
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let mut file = request
        .open_file(
            input(FileId(0), b"[bc", &elements, &[true], &no_catalog),
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
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"ab(");
    let mut file = request
        .open_file(
            input(FileId(0), b"ab(", &elements, &[true], &no_catalog),
            &mut symbols,
        )
        .unwrap();
    let child_error = regex_error("[");
    assert_ne!(
        child_error,
        regex_error("("),
        "child and later head failures must be distinguishable"
    );
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
    assert!(
        work.iter().any(|fed| *fed != [0; 3]),
        "first attempt must reach actual continuation work"
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
fn external_delivery_refusal_is_retryable_but_the_later_head_error_is_terminal() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::DynamicRegex]);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"ab(");
    let first = expected(FileId(0), &elements, &[1]);
    let mut file = request
        .open_file(
            input(FileId(0), b"ab(", &elements, &[true], &no_catalog),
            &mut symbols,
        )
        .unwrap();
    let mut attempts = 0;
    let refused = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        attempts += 1;
        assert_eq!(row, first.cells);
        Err(OperatingError::WrongBinding)
    });
    assert!(matches!(refused, Err(OperatingError::WrongBinding)));
    assert_eq!(attempts, 1);
    counts(&file, 1, 4);
    let mut delivered = vec![];
    let retried = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        delivered.push(row.to_vec());
        Ok(())
    });
    assert!(matches!(retried,Err(ref error) if *error==regex_error("(")));
    assert_eq!(delivered, vec![first.cells]);
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
