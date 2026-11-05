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
    let request = engine.begin(&program, &corpus);
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
    let request = engine.begin(&program, &corpus);
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
    let request = engine.begin(&program, &corpus);
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

#[test]
fn shared_walk_uses_the_same_join_dependency_contract_for_callback_delivery() {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let (engine, _) = engine();
    let mut authored = authored_request(2);
    authored.plans[0].steps = vec![
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
    authored.plans[0].emit.step = StepId(2);
    authored.plans[1].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::SyntaxTree),
        comparisons: selection(Selector::Kind("other")),
        kept: vec![],
    };
    let program = engine.prepare(authored).unwrap();
    for diagnostic in [DiagnosticDelivery::Collected, DiagnosticDelivery::Forwarded] {
        for independent_first in [false, true] {
            let corpus = corpus();
            let request = engine.begin(&program, &corpus);
            let (elements, mut symbols) = tree(FileId(0), b"abc");
            let intents = [
                FileRuleIntent::Run {
                    diagnostic,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                },
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                },
            ];
            let mut file = request
                .open_file(
                    input(FileId(0), b"abc", &elements, &intents, &no_catalog),
                    &mut symbols,
                )
                .unwrap();
            if independent_first {
                assert_eq!(
                    complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
                    expected(FileId(0), &elements, &[2])
                );
            }
            for attempt in 0..2 {
                match diagnostic {
                    DiagnosticDelivery::Collected => {
                        let result = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
                        assert!(
                            matches!(result.emit, RootOutcome::Failed(ref error) if *error == regex_error("["))
                        );
                    }
                    DiagnosticDelivery::Forwarded => {
                        let mut delivered = Vec::new();
                        let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                            delivered.push(row.to_vec());
                            Ok(())
                        });
                        assert!(matches!(result, Err(ref error) if *error == regex_error("[")));
                        assert!(
                            delivered.is_empty(),
                            "a failed canonical Join commits no rows"
                        );
                    }
                    DiagnosticDelivery::Disabled => {
                        unreachable!("test declares a diagnostic consumer")
                    }
                }
                if attempt == 0 && !independent_first {
                    assert_eq!(
                        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
                        expected(FileId(0), &elements, &[2])
                    );
                }
                // The independent selector and LHS reach the first actual head
                // cohort. Only LHS publication unlocks the Join's demanded RHS,
                // which reaches a second cohort. A terminal retry adds no work.
                counts(&file, 2, 8);
                assert_eq!(
                    file.walk_counts().predicate_rows.0,
                    2,
                    "the independent predicate and first failing RHS predicate each run once"
                );
            }
            file.finish().unwrap();
            request.finish();
        }
    }
}

#[test]
fn independent_semantic_relation_selectors_share_one_source_walk() {
    use crate::boundary_fixture::named_functions;
    use ryo_relations::domain::relations::{
        service::node_id,
        types::{CellKind, Position, Value},
    };
    let (engine, semantics) = engine();
    let mut authored = authored_request(2);
    for (index, name) in ["main", "other"].iter().enumerate() {
        authored.plans[index].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons: vec![compared(Test::Equal, 1, text(name))],
            kept: vec![ColumnIndex(0)],
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let table = named_functions(&mut symbols);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        Ok(SourceRows::Computed(&table))
    };
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }; 2];
    let mut file = request
        .open_file(
            input(FileId(0), b"mainother", &elements, &intents, &read),
            &mut symbols,
        )
        .unwrap();
    for (rule, positions) in [(0, &[0_u32, 1][..]), (1, &[2_u32][..])] {
        assert_eq!(
            complete(file.rule(RuleIndex(rule), demand(), &mut symbols).unwrap()),
            Table {
                kinds: vec![CellKind::Node],
                count: RowCount(positions.len()),
                cells: positions
                    .iter()
                    .map(|p| Value::Node(node_id(FileId(0), Position(*p))))
                    .collect(),
            },
        );
    }
    counts(&file, 1, 3);
    assert_eq!(file.walk_counts().predicate_rows.0, 6);
    assert_eq!(
        semantics.selects(),
        0,
        "both selectors use shared row dispatch"
    );
    file.finish().unwrap();
    request.finish();
}

#[test]
fn frontier_and_callback_share_the_same_atomic_semantic_failure() {
    use ryo_relations::domain::relations::types::{Words, WorkerNumber};
    for capture_first in [true, false] {
        let (engine, _) = engine();
        let mut authored = authored_request(1);
        authored.plans[0].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: selection(Selector::DynamicRegex),
            kept: vec![],
        };
        authored.plans[0].frontier = vec![Input::Step(StepId(0))];
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
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
                            arena: &mut arena,
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
    let request = engine.begin(&program, &corpus);
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

fn conditional_join_request(count: usize) -> PreparedRequest {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let mut authored = authored_request(count);
    authored.plans[0].steps = vec![
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
    authored.plans[0].emit.step = StepId(2);
    for index in 1..count {
        authored.plans[index] = authored.plans[0].clone();
    }
    authored
}

#[test]
fn callback_and_frontier_use_one_dependency_contract_in_both_orders() {
    use ryo_relations::domain::relations::types::{Words, WorkerNumber};
    let (engine, _) = engine();
    let mut authored = conditional_join_request(1);
    authored.plans[0].frontier = vec![Input::Step(StepId(2))];
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
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
                            arena: &mut arena,
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
