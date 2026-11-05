//! Both declared Union populations drive the same owner-held invocation.
use super::*;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::HostedNumber,
    records::types::Cell as HostCell,
};

fn program(engine: &ExecutionUseCaseImpl, left: &[i64], right: &[i64]) -> PreparedProgram {
    let mut input = authored_request(1);
    input.plans[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(1),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
            },
        },
    ];
    input.plans[0].emit.step = StepId(2);
    input.hosted = [left, right]
        .into_iter()
        .enumerate()
        .map(|(side, rows)| HostRelation {
            name: HostName(format!("union-side-{side}")),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: rows
                .iter()
                .map(|value| HostRow(vec![HostCell::Number(Integer(*value))]))
                .collect(),
        })
        .collect();
    engine.prepare(input).expect("actual Union program")
}

#[test]
fn union_consumes_both_populations_in_order_with_one_distinct_scope() {
    for (left, right, expected) in [
        (&[][..], &[2, 1, 2][..], &[2, 1][..]),
        (&[3, 2, 3][..], &[][..], &[3, 2][..]),
        (&[3, 2, 3][..], &[2, 1, 3, 1][..], &[3, 2, 1][..]),
        (&[][..], &[][..], &[][..]),
    ] {
        let engine = engine();
        let prepared = program(&engine, left, right);
        let corpus = corpus();
        let request = engine.begin(&prepared, &corpus);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
        for forwarded in [false, true] {
            let mut symbols = Symbols::default();
            let mut file = request
                .open_file(
                    FileInput {
                        language: SourceLanguage::Recognized,
                        resident_stability: facts::ResidentStability::Stable,
                        file: FileId(0),
                        evaluation: &[],
                        publication: &[],
                        demanded: &[true],
                        source: b"abc",
                        elements: &elements,
                        read: &read,
                        semantic: None,
                        kinds: None,
                    },
                    &mut symbols,
                )
                .expect("open Union file");
            let mut cells = vec![];
            if forwarded {
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    cells.extend_from_slice(row);
                    Ok(())
                })
                .expect("forward both Union populations");
            } else {
                let RootOutcome::Complete(table) = file
                    .rule(RuleIndex(0), demand(), &mut symbols)
                    .expect("collect both Union populations")
                    .emit
                else {
                    panic!("complete Union output")
                };
                assert_eq!(table.kinds, [CellKind::Number]);
                assert_eq!(table.count, RowCount(expected.len()));
                cells = table.cells;
            }
            assert_eq!(
                cells,
                expected
                    .iter()
                    .map(|v| Value::Number(Integer(*v)))
                    .collect::<Vec<_>>()
            );
            file.finish().expect("release Union file");
        }
        request.finish();
    }
}

#[test]
fn union_stops_on_right_delivery_refusal_and_retries_with_a_new_distinct_scope() {
    let engine = engine();
    let prepared = program(&engine, &[3], &[2, 1, 2]);
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                demanded: &[true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("open Union retry file");
    let mut calls = 0;
    let mut prefix = vec![];
    let refused = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        calls += 1;
        if calls == 2 {
            return Err(OperatingError::WrongBinding);
        }
        prefix.extend_from_slice(row);
        Ok(())
    });
    assert!(matches!(refused, Err(OperatingError::WrongBinding)));
    assert_eq!(
        calls, 2,
        "no later right row after the first refused delivery"
    );
    assert_eq!(prefix, [Value::Number(Integer(3))]);
    let mut retried = vec![];
    file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        retried.extend_from_slice(row);
        Ok(())
    })
    .expect("retry Union delivery");
    assert_eq!(retried, [3, 2, 1].map(|v| Value::Number(Integer(v))));
    file.finish().expect("release retried Union");
    request.finish();
}

#[test]
fn union_preserves_left_source_refusal_and_demands_right_after_empty_left() {
    for left_refuses in [true, false] {
        let engine = engine();
        let mut input = authored_request(1);
        input.plans[0].steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::SemanticChild),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Union {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                },
            },
        ];
        input.plans[0].emit.step = StepId(2);
        let prepared = engine.prepare(input).expect("source-bound Union");
        let corpus = corpus();
        let request = engine.begin(&prepared, &corpus);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let empty = Table {
            kinds: kinds(Relation::Function).to_vec(),
            count: RowCount(0),
            cells: vec![],
        };
        let left_reads = Cell::new(0);
        let right_reads = Cell::new(0);
        let read = |relation| match relation {
            Relation::Function => {
                left_reads.set(left_reads.get() + 1);
                if left_refuses {
                    Err(OperatingError::WrongBinding)
                } else {
                    Ok(SourceRows::Computed(&empty))
                }
            }
            Relation::SemanticChild => {
                right_reads.set(right_reads.get() + 1);
                Err(OperatingError::NotRead)
            }
            _ => Err(OperatingError::WrongOperation),
        };
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    demanded: &[true],
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .expect("open refused Union");
        let result = file.rule(RuleIndex(0), demand(), &mut symbols);
        assert!(if left_refuses {
            matches!(result, Err(OperatingError::WrongBinding))
        } else {
            matches!(result, Err(OperatingError::NotRead))
        });
        assert_eq!(left_reads.get(), 1);
        assert_eq!(
            right_reads.get(),
            usize::from(!left_refuses),
            "left failure wins; empty success still demands the right side"
        );
        file.finish().expect("release refused Union");
        request.finish();
    }
}
