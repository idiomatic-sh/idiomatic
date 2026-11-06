//! Selected rows, joins and escaped frontiers use the actual public file owner.
use super::super::fixtures::{complete, input};
use crate::boundary_fixture::{authored_request, corpus, demand, engine};
use ryo_execution::inbound::use_cases::execution::{interface::*, types::*};
use ryo_language::domain::{
    elements::types::Elements,
    symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    },
};
use ryo_relations::domain::{
    relations::{
        service::{kinds, node_id, to_bits},
        types::*,
    },
    terms::types::NameText,
};
use ryo_values::domain::{
    plan_parts::types::*,
    plans::types::*,
    rows::types::{RowCount, Table},
    stored_facts::types::Bucket,
    table_words::service::decode_complete,
};
use std::cell::Cell;

fn selected_join(distinct: bool) -> Plan {
    let mut plan = authored_request(1).plans.remove(0);
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                kept: vec![ColumnIndex(0), ColumnIndex(2)],
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("keep".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(u32::from(distinct))],
                distinct: if distinct {
                    RepeatedRows::Dropped
                } else {
                    RepeatedRows::Kept
                },
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(0)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(u32::from(distinct))],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    plan.emit.step = StepId(2);
    plan.emit.subject = ColumnIndex(1);
    plan.frontier = vec![Input::Step(StepId(0))];
    plan
}

fn functions(symbols: &mut Symbols) -> Table {
    let keep = intern_derived(symbols, ByteSlice(b""), NameText("keep"));
    let skip = intern_derived(symbols, ByteSlice(b""), NameText("skip"));
    Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(64),
        cells: (0..64)
            .flat_map(|row| {
                [
                    Value::Node(node_id(FileId(0), Position(row))),
                    Value::Symbol(if row % 8 == 0 { keep } else { skip }),
                    Value::File(FileId(0)),
                    Value::Span(Span {
                        start: Offset(row),
                        end: Offset(row + 1),
                    }),
                ]
            })
            .collect(),
    }
}

#[test]
fn selected_join_shares_work_preserves_counts_and_exports_owned_frontiers() {
    for distinct in [false, true] {
        for stored in [false, true] {
            let engine = engine();
            let mut authored = authored_request(2);
            authored.plans = vec![selected_join(distinct); 2];
            let program = engine.prepare(authored).unwrap();
            let corpus = corpus();
            let request = engine.begin(&program, &corpus);
            let mut symbols = Symbols::default();
            let source = functions(&mut symbols);
            let columns: Vec<FactColumn> = (0..source.kinds.len())
                .map(|column| {
                    FactColumn(
                        source
                            .cells
                            .chunks(source.kinds.len())
                            .map(|row| to_bits(row[column]).0)
                            .collect(),
                    )
                })
                .collect();
            let reads = Cell::new(0);
            let read = |relation| {
                reads.set(reads.get() + 1);
                if relation == Relation::Function {
                    Ok(if stored {
                        SourceRows::Stored(ColumnsView {
                            kinds: kinds(Relation::Function),
                            columns: &columns,
                        })
                    } else {
                        SourceRows::Computed(&source)
                    })
                } else {
                    Err(OperatingError::NotRead)
                }
            };
            let elements = Elements::default();
            let intents = [FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }; 2];
            let mut file = request
                .open_file(
                    input(FileId(0), b"", &elements, &intents, &read),
                    &mut symbols,
                )
                .unwrap();
            let expected = Table {
                kinds: vec![
                    if distinct {
                        CellKind::File
                    } else {
                        CellKind::Node
                    },
                    CellKind::File,
                ],
                count: RowCount(8),
                cells: (0..8)
                    .flat_map(|row| {
                        [
                            if distinct {
                                Value::File(FileId(0))
                            } else {
                                Value::Node(node_id(FileId(0), Position(row * 8)))
                            },
                            Value::File(FileId(0)),
                        ]
                    })
                    .collect(),
            };
            let expected_frontier = Table {
                kinds: vec![CellKind::Node, CellKind::File],
                count: RowCount(8),
                cells: (0..8)
                    .flat_map(|row| {
                        [
                            Value::Node(node_id(FileId(0), Position(row * 8))),
                            Value::File(FileId(0)),
                        ]
                    })
                    .collect(),
            };
            let first = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            let counts = first.counts.fed.to_vec();
            assert_eq!(
                counts.iter().map(|inputs| inputs.0).collect::<Vec<_>>(),
                vec![[64, 0, 0], [8, 0, 0], [if distinct { 1 } else { 8 }, 8, 0]],
                "mandatory logical work counts preserve each authored operator input"
            );
            assert_eq!(complete(first), expected);
            let work = file.readiness_counts();
            let walks = file.walk_counts();
            let reads_after_run = reads.get();
            let mut escaped = Vec::new();
            for rule in [RuleIndex(0), RuleIndex(1)] {
                let before_delivery = reads.get();
                let outcome = file.rule(rule, demand(), &mut symbols).unwrap();
                assert_eq!(outcome.counts.fed.as_ref(), counts.as_slice());
                assert_eq!(complete(outcome), expected);
                assert_eq!(
                    reads.get(),
                    before_delivery,
                    "publication replay does not borrow a provider"
                );
                let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
                let captures = file
                    .capture_frontiers(
                        rule,
                        Bucket(0),
                        &mut symbols,
                        CaptureContext {
                            worker: WorkerNumber(0),
                            arena: &mut arena,
                            places: &mut places,
                            words: &mut words,
                        },
                    )
                    .unwrap();
                assert_eq!(captures.len(), 1);
                assert_eq!(
                    reads.get(),
                    before_delivery + usize::from(stored),
                    "capture reborrows Stored selection backing once; Computed backing is already owned"
                );
                assert_eq!(
                    file.readiness_counts(),
                    work,
                    "capture cannot reevaluate the selection or Join"
                );
                assert_eq!(file.walk_counts(), walks);
                assert_eq!(file.counts(rule).unwrap().fed.as_ref(), counts.as_slice());
                escaped.extend(captures.into_iter().map(|capture| capture.outcome.unwrap()));
            }
            assert_eq!(
                file.readiness_counts(),
                work,
                "delivery must not re-execute dependencies"
            );
            assert_eq!(file.walk_counts(), walks);
            assert_eq!(
                reads.get(),
                reads_after_run + escaped.len() * usize::from(stored)
            );
            file.finish().unwrap();
            // The actual encoded publication must survive its file execution owner.
            for frontier in escaped {
                let decoded = decode_complete(&frontier.words, &mut Vec::new()).unwrap();
                assert_eq!(decoded, expected_frontier);
            }
            // A new generation cannot answer an unavailable provider from the previous file cache.
            let refused = |_| Err(OperatingError::NotRead);
            let mut next = request
                .open_file(
                    input(FileId(0), b"", &elements, &intents, &refused),
                    &mut symbols,
                )
                .unwrap();
            for rule in [RuleIndex(0), RuleIndex(1)] {
                let result = next.rule(rule, demand(), &mut symbols).unwrap();
                assert!(matches!(
                    result.emit,
                    RootOutcome::Failed(OperatingError::NotRead)
                ));
            }
            next.finish().unwrap();
            request.finish();
        }
    }
}

#[test]
fn grouped_dependency_and_original_source_join_in_both_input_positions() {
    for reversed in [false, true] {
        let engine = engine();
        let mut authored = authored_request(1);
        let source = Input::Relation(Relation::LanguageTarget);
        authored.plans[0].steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: source,
                    kept: vec![],
                    comparisons: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::GroupCount {
                    from: Input::Step(StepId(0)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Step(StepId(1)),
                    kept: vec![],
                    comparisons: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: if reversed {
                        source
                    } else {
                        Input::Step(StepId(2))
                    },
                    right: if reversed {
                        Input::Step(StepId(2))
                    } else {
                        source
                    },
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: vec![
                        JoinColumn {
                            side: if reversed { Side::Left } else { Side::Right },
                            column: ColumnIndex(1),
                        },
                        JoinColumn {
                            side: if reversed { Side::Right } else { Side::Left },
                            column: ColumnIndex(1),
                        },
                    ],
                    filter: vec![],
                    view: vec![],
                    reach: None,
                },
            },
        ];
        authored.plans[0].emit.step = StepId(3);
        authored.plans[0].emit.subject = ColumnIndex(0);
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let columns = [
            FactColumn(vec![to_bits(Value::File(FileId(0))).0; 2]),
            FactColumn(vec![
                to_bits(Value::Number(Integer(3))).0,
                to_bits(Value::Number(Integer(5))).0,
            ]),
            FactColumn(vec![
                to_bits(Value::Number(Integer(7))).0,
                to_bits(Value::Number(Integer(9))).0,
            ]),
        ];
        let read = |relation| {
            if relation == Relation::LanguageTarget {
                Ok(SourceRows::Stored(ColumnsView {
                    kinds: kinds(relation),
                    columns: &columns,
                }))
            } else {
                Err(OperatingError::NotRead)
            }
        };
        let elements = Elements::default();
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
        let mut file = request
            .open_file(
                input(FileId(0), b"", &elements, &intents, &read),
                &mut symbols,
            )
            .unwrap();
        let outcome = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        assert_eq!(
            complete(outcome),
            Table {
                kinds: vec![CellKind::Number, CellKind::Number],
                count: RowCount(2),
                cells: vec![
                    Value::Number(Integer(3)),
                    Value::Number(Integer(2)),
                    Value::Number(Integer(5)),
                    Value::Number(Integer(2))
                ],
            }
        );
        file.finish().unwrap();
        request.finish();
    }
}

#[test]
fn indexed_join_counts_actual_matching_rhs_visits_and_does_no_work_on_replay() {
    use ryo_values::domain::{
        hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
        row_texts::types::Cell as HostCell,
    };
    for (left, expected, visits) in [
        (
            vec![7, 7, 9],
            vec![(7, 100), (7, 200), (7, 100), (7, 200)],
            4,
        ),
        (vec![99, 99], vec![], 0),
    ] {
        let engine = engine();
        let mut authored = authored_request(1);
        authored.hosted = vec![
            HostRelation {
                name: HostName("left-keys".into()),
                columns: vec![HostColumn {
                    name: HostColumnName("key".into()),
                    kind: HostKind::Number,
                }],
                rows: left
                    .iter()
                    .map(|key| HostRow(vec![HostCell::Number(Integer(*key))]))
                    .collect(),
            },
            HostRelation {
                name: HostName("right-values".into()),
                columns: ["key", "value"]
                    .map(|name| HostColumn {
                        name: HostColumnName(name.into()),
                        kind: HostKind::Number,
                    })
                    .to_vec(),
                rows: [(7, 100), (7, 200), (8, 300), (10, 400), (11, 500)]
                    .into_iter()
                    .map(|(key, value)| {
                        HostRow(vec![
                            HostCell::Number(Integer(key)),
                            HostCell::Number(Integer(value)),
                        ])
                    })
                    .collect(),
            },
        ];
        let plan = &mut authored.plans[0];
        plan.steps = [0, 1]
            .map(|relation| PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(relation),
                },
            })
            .to_vec();
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        });
        plan.emit.step = StepId(2);
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let mut symbols = Symbols::default();
        let elements = Elements::default();
        let read = |_| Err(OperatingError::NotRead);
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
        let mut file = request
            .open_file(
                input(FileId(0), b"", &elements, &intents, &read),
                &mut symbols,
            )
            .unwrap();
        let first = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        assert_eq!(first.counts.fed[2].0, [left.len() as u64, visits, 0]);
        let counts = first.counts.fed.to_vec();
        let expected = Table {
            kinds: vec![CellKind::Number, CellKind::Number],
            count: RowCount(expected.len()),
            cells: expected
                .into_iter()
                .flat_map(|(key, value)| {
                    [Value::Number(Integer(key)), Value::Number(Integer(value))]
                })
                .collect(),
        };
        assert_eq!(complete(first), expected);
        let readiness = file.readiness_counts();
        let again = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        assert_eq!(again.counts.fed.as_ref(), counts.as_slice());
        assert_eq!(complete(again), expected);
        assert_eq!(file.readiness_counts(), readiness);
        file.finish().unwrap();
        request.finish();
    }
}
