//! Shared cached prefixes preserve authored error order and the existing retry protocol.
use super::{fixture::*, no_catalog};
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Integer, Relation, Value},
};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{ColumnIndex, HostedNumber, Input, StepId, Test},
    plans::types::{Operation, PlanStep, StepScope},
    records::types::Cell,
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

fn readiness_intent() -> FileRuleIntent {
    FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }
}
fn readiness_chain() -> Vec<PlanStep> {
    let mut steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
    }];
    for (left, right) in [(0, 0), (1, 0), (1, 2)] {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(StepId(left)),
                right: Input::Step(StepId(right)),
            },
        });
    }
    for _ in 0..96 {
        let prior = StepId(u32::try_from(steps.len() - 1).unwrap());
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(prior),
                right: Input::Step(prior),
            },
        });
    }
    steps
}
fn readiness_hosted() -> HostRelation {
    HostRelation {
        name: HostName("numbers".into()),
        columns: vec![HostColumn {
            name: HostColumnName("number".into()),
            kind: HostKind::Number,
        }],
        rows: [3, 1, 3, 2]
            .into_iter()
            .map(|n| HostRow(vec![Cell::Number(Integer(n))]))
            .collect(),
    }
}
fn readiness_expected() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        count: RowCount(3),
        cells: [3, 1, 2]
            .into_iter()
            .map(|n| Value::Number(Integer(n)))
            .collect(),
    }
}
#[test]
fn diamond_deep_chain_and_duplicate_roots_wake_only_actual_waiters() {
    for empty in [false, true] {
        exercise_readiness_chain(empty);
    }
}
fn exercise_readiness_chain(empty: bool) {
    let (engine, _) = engine();
    let mut authored = authored_request(3);
    let steps = readiness_chain();
    let length = steps.len() as u64;
    for (index, plan) in authored.plans.iter_mut().enumerate() {
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
    let request = engine.begin(&prepared, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [readiness_intent(), readiness_intent(), readiness_intent()];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(file.readiness_counts().preparations.0, 0);
    assert_eq!(
        complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap()),
        expected.clone()
    );
    let work = file.readiness_counts();
    assert!(
        work.suspensions.0 >= 96,
        "the actual deep chain reached demanded dependencies: {work:?}"
    );
    assert!(
        work.preparations.0 <= 4 * length,
        "blocked ancestors must not be rescanned: {work:?}"
    );
    assert!(
        work.evaluations.0 <= 2 * length,
        "one evaluation per distinct physical region: {work:?}"
    );
    assert_eq!(work.wakeups, work.suspensions);
    assert_eq!(
        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
        expected.clone()
    );
    assert_eq!(
        file.readiness_counts(),
        work,
        "the duplicate logical root replays the same publication"
    );
    assert_eq!(
        complete(file.rule(RuleIndex(2), demand(), &mut symbols).unwrap()),
        expected.clone()
    );
    assert_eq!(
        file.readiness_counts(),
        work,
        "the exact duplicate terminal is not polled twice"
    );
    file.finish().unwrap();
    request.finish();
}
#[test]
fn failed_shared_producer_wakes_both_roots_and_does_not_erase_independent_output() {
    let (engine, _) = engine();
    let mut authored = authored_request(3);
    let mut failing = readiness_chain();
    failing[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![],
    };
    for (index, plan) in authored.plans[..2].iter_mut().enumerate() {
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
    authored.plans[2].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
    }];
    authored.hosted = vec![readiness_hosted()];
    let prepared = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let calls = std::cell::Cell::new(0);
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        calls.set(calls.get() + 1);
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
        assert!(matches!(
            file.rule(RuleIndex(index), demand(), &mut symbols)
                .unwrap()
                .emit,
            RootOutcome::Unknown(OperatingError::NotRead)
        ));
    }
    assert_eq!(
        calls.get(),
        1,
        "one actual failed source admission, no waiting-parent retries"
    );
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
    let work = file.readiness_counts();
    assert!(
        work.preparations.0 <= 4 * failing.len() as u64,
        "failure unwinds each waiter once: {work:?}"
    );
    assert_eq!(work.wakeups, work.suspensions);
    file.finish().unwrap();
    request.finish();
}
