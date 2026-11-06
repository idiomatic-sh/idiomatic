//! Fault injection is a real semantic dependency; every execution uses FileRun.
use super::super::cases::test::{node, table_of, tree_rows, wide_tree};
use super::super::*;
use ryo_operators::inbound::use_cases::operators::interface::{
    JoinInput, JoinRow, OperatorAnswer, OperatorCall, OperatorsUseCase, PortableOperatorFault,
};
use std::sync::atomic::{AtomicUsize, Ordering};
pub(in super::super) const REFUSAL: u32 = 60001;

#[derive(Debug, Default)]
pub(in super::super) struct Witness {
    pub(in super::super) calls: AtomicUsize,
    pub(in super::super) columns: AtomicUsize,
    pub(in super::super) positions: AtomicUsize,
    pub(in super::super) refuse: bool,
    pub(in super::super) only_columns: bool,
}
impl OperatorsUseCase for Witness {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if let OperatorCall::EqualityJoin { input } = &call {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let columnar = matches!(
                input,
                JoinInput::Values {
                    left: JoinRow::Columns(_),
                    ..
                }
            );
            match input {
                JoinInput::Values {
                    left: JoinRow::Columns(_),
                    ..
                } => {
                    self.columns.fetch_add(1, Ordering::Relaxed);
                }
                JoinInput::Positions { .. } => {
                    self.positions.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            if self.refuse && (!self.only_columns || columnar) {
                return Err(PortableOperatorFault::NoSuchColumn { index: REFUSAL });
            }
        }
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
            .operators(call)
    }
}
fn refusal(result: Result<Table, OperatingError>) {
    assert!(
        matches!(
            result,
            Err(OperatingError::Table(
                ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: REFUSAL }
            ))
        ),
        "{result:?}"
    );
}
fn checked_refusal(op: &Operation, rows: [Rows<'_>; 2], symbols: &mut Symbols) {
    for delivery in [false, true] {
        for optional_limit in [None, Some(0)] {
            let witness = Arc::new(Witness {
                refuse: true,
                ..Witness::default()
            });
            let observed = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                witness.clone(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .expect("real compiled owner");
            refusal(observed.result);
            assert!(observed.prefix.is_empty());
            assert_eq!(witness.calls.load(Ordering::Relaxed), 1);
            // A refusing dependency does not alter the immutable compiled meaning.
            let accepted = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                real_port(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .unwrap();
            assert!(accepted.result.unwrap().count.0 > 0);
        }
    }
}
#[test]
fn computed_stored_compact_and_tree_providers_reach_the_same_required_capability() {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right = table_of(
        &[CellKind::Number, CellKind::Number],
        &[
            &[Cell::Number(Integer(7)), Cell::Number(Integer(1))],
            &[Cell::Number(Integer(7)), Cell::Number(Integer(2))],
        ],
    );
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let columns = [FactColumn(vec![7, 7]), FactColumn(vec![1, 2])];
    for rows in [
        Rows::Computed(&right),
        Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number; 2],
            columns: &columns,
        }),
    ] {
        checked_refusal(&op, [Rows::Computed(&left), rows], &mut Symbols::default());
    }
    let mut words = ryo_relations::domain::relations::types::Words::default();
    encode_table(&right, &[], &mut words);
    let (compact, names) = admit_compact(
        Arc::new(KeptWords(pack(WordSlice(&words.0)))),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .unwrap();
    assert!(names.is_empty());
    checked_refusal(
        &op,
        [Rows::Computed(&left), Rows::Compact(&compact)],
        &mut Symbols::default(),
    );

    let (elements, mut symbols) = wide_tree(1, true);
    let nodes = table_of(&[CellKind::Node], &[&[node(1)]]);
    let op = operation(&[0], &[0], &[(Side::Right, 0)], vec![], &[]);
    checked_refusal(
        &op,
        [
            Rows::Computed(&nodes),
            tree_rows(&elements, TreeForm::Elements),
        ],
        &mut symbols,
    );
    checked_refusal(
        &op,
        [
            tree_rows(&elements, TreeForm::Elements),
            Rows::Computed(&nodes),
        ],
        &mut symbols,
    );
}
#[test]
fn empty_or_failed_left_never_enters_the_join_capability_or_builds_a_rhs_index() {
    let left = table_of(&[CellKind::Number], &[]);
    let right = table_of(
        &[CellKind::Number, CellKind::Number],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    for failure in [None, Some(OperatingError::NotRun)] {
        for delivery in [false, true] {
            let witness = Arc::new(Witness {
                refuse: true,
                ..Witness::default()
            });
            let actual = invoke(
                &op,
                [Rows::Computed(&left), Rows::Computed(&right)],
                &mut Symbols::default(),
                b"",
                delivery,
                witness.clone(),
                Faults {
                    left: failure.clone(),
                    ..Faults::default()
                },
            )
            .unwrap();
            match &failure {
                Some(_) => assert!(matches!(actual.result, Err(OperatingError::NotRun))),
                None => assert!(actual.result.unwrap().cells.is_empty()),
            }
            assert_eq!(witness.calls.load(Ordering::Relaxed), 0);
            if let Some(error) = &failure {
                assert_eq!(actual.attempts, [SourceAttempt::Refused(0, error.clone())]);
                assert_eq!(actual.reads, [1, 0]);
            } else {
                assert_eq!(actual.attempts.first(), Some(&SourceAttempt::Available(0)));
                assert!(
                    actual
                        .attempts
                        .iter()
                        .all(|attempt| matches!(attempt, SourceAttempt::Available(_)))
                );
                assert!(
                    actual.attempts.contains(&SourceAttempt::Available(1)),
                    "canonical dependency admission still checks RHS after a successful empty LHS"
                );
            }
            assert_eq!(actual.work.index_rows, 0);
            assert_eq!(actual.work.index_cells, 0);
            assert!(actual.prefix.is_empty());
        }
    }
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults {
            right: Some(OperatingError::NotRead),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(matches!(actual.result, Err(OperatingError::NotRead)));
    assert_eq!(actual.work.index_rows, 0);
}

#[derive(Debug)]
struct KeepsCalling {
    refuse_project: bool,
}
impl OperatorsUseCase for KeepsCalling {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match call {
            OperatorCall::Project { .. } if self.refuse_project => {
                Err(PortableOperatorFault::NoSuchColumn { index: REFUSAL })
            }
            OperatorCall::EqualityJoin {
                input: JoinInput::Values { driver, .. },
            } => {
                if self.refuse_project {
                    assert!(driver.emit(&[Cell::Number(Integer(1))]).is_err());
                    assert!(driver.emit(&[Cell::Number(Integer(2))]).is_err());
                    assert!(driver.next(&[Cell::Number(Integer(7))]).is_err());
                } else {
                    // External delivery begins only after this semantic result
                    // commits; it cannot interrupt this driver retroactively.
                    assert!(driver.emit(&[Cell::Number(Integer(1))]).is_ok());
                    assert!(driver.emit(&[Cell::Number(Integer(2))]).is_ok());
                }
                Ok(OperatorAnswer::Written)
            }
            call => {
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
                    .operators(call)
            }
        }
    }
}

#[test]
fn sink_failure_wins_after_a_kernel_commits_multiple_rows_and_reports_success() {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right = table_of(
        &[CellKind::Number, CellKind::Number],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let actual = invoke(
        &operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]),
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        Arc::new(KeepsCalling {
            refuse_project: false,
        }),
        Faults {
            sink_after: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(matches!(actual.result, Err(OperatingError::NotRun)));
    assert!(actual.prefix.is_empty());
}

#[test]
fn fused_semantic_refusal_stays_sticky_when_the_kernel_continues_and_reports_success() {
    let left_table = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right_table = table_of(
        &[CellKind::Number; 2],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let mut steps = vec![];
    let (left, l) = provider(Rows::Computed(&left_table), 0, &mut steps);
    let (right, r) = provider(Rows::Computed(&right_table), 1, &mut steps);
    let join = steps.len() as u32;
    let mut op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let Operation::Join {
        left: from,
        right: other,
        ..
    } = &mut op
    else {
        unreachable!()
    };
    *from = l;
    *other = r;
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: op,
    });
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: PlanInput::Step(StepId(join)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    });
    let mut request = plan(Scenario::ProjectRetry);
    request.emit.step = StepId((steps.len() - 1) as u32);
    request.steps = steps;
    let actual = execute(
        request,
        [left, right],
        &mut Symbols::default(),
        b"",
        true,
        Arc::new(KeepsCalling {
            refuse_project: true,
        }),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(
        actual.instructions, 2,
        "the failure must occur in the real fused Join→Project continuation"
    );
    refusal(actual.result);
    assert!(
        actual.prefix.is_empty(),
        "failed semantic publication delivers no prefix"
    );
}

#[test]
fn compiled_tree_and_step_joins_cannot_bypass_the_same_semantic_refusal() {
    for right in [
        PlanInput::Relation(Relation::SyntaxTree),
        PlanInput::Relation(Relation::SyntaxChild),
        PlanInput::Step(StepId(1)),
    ] {
        for refuse in [false, true] {
            let (elements, mut symbols) = wide_tree(1, true);
            let select = || PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxTree),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            };
            let mut request = plan(Scenario::ProjectRetry);
            request.steps = vec![select()];
            if matches!(right, PlanInput::Step(_)) {
                request.steps.push(select());
            }
            let target = StepId(request.steps.len() as u32);
            let mut join = operation(
                &[0],
                &[if right == PlanInput::Relation(Relation::SyntaxChild) {
                    1
                } else {
                    0
                }],
                &[(Side::Left, 0)],
                vec![],
                &[],
            );
            let Operation::Join {
                left, right: rhs, ..
            } = &mut join
            else {
                unreachable!()
            };
            *left = PlanInput::Step(StepId(0));
            *rhs = right;
            request.steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: join,
            });
            request.emit.step = target;
            let (left, _) = provider(tree_rows(&elements, TreeForm::Elements), 0, &mut vec![]);
            let (other, _) = provider(tree_rows(&elements, TreeForm::Edges), 1, &mut vec![]);
            let witness = Arc::new(Witness {
                refuse,
                ..Witness::default()
            });
            let actual = execute(
                request,
                [left, other],
                &mut symbols,
                b"abcd",
                false,
                witness.clone(),
                Faults::default(),
            )
            .unwrap();
            assert!(
                actual.native_prepared,
                "tree/step case must install the real compiled native strategy"
            );
            assert_eq!(actual.work.native_positions, usize::from(!refuse));
            assert_eq!(
                witness.positions.load(Ordering::Relaxed),
                if refuse { 1 } else { 4 },
                "one semantic call per authored left tree position, stopping at the first refusal"
            );
            assert_eq!(witness.columns.load(Ordering::Relaxed), 0);
            if refuse {
                refusal(actual.result);
            } else {
                let expected = if right == PlanInput::Relation(Relation::SyntaxChild) {
                    vec![node(1), node(2), node(3)]
                } else {
                    vec![node(0), node(1), node(2), node(3)]
                };
                assert_eq!(actual.result.unwrap().cells, expected);
            }
        }
    }
}

#[test]
fn native_provenance_preserves_canonical_left_then_right_refusal_even_when_left_is_empty() {
    use ryo_values::domain::plan_parts::types::{
        Comparison, LiteralText, Operand, QueryLiteral, Test,
    };
    for (left_fails, right_fails, error) in [
        (false, true, Some(OperatingError::NotRead)),
        (true, true, Some(OperatingError::NotRun)),
        (true, false, Some(OperatingError::NotRun)),
        (false, false, None),
    ] {
        let (elements, mut symbols) = wide_tree(1, true);
        let mut request = plan(Scenario::ProjectRetry);
        request.steps = vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                kept: vec![ColumnIndex(0)],
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("absent-kind".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
            },
        }];
        let mut join = operation(&[0], &[1], &[(Side::Left, 0)], vec![], &[]);
        let Operation::Join { left, right, .. } = &mut join else {
            unreachable!()
        };
        *left = PlanInput::Step(StepId(0));
        *right = PlanInput::Relation(Relation::SyntaxChild);
        request.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: join,
        });
        request.emit.step = StepId(1);
        let (left, _) = provider(tree_rows(&elements, TreeForm::Elements), 0, &mut vec![]);
        let (right, _) = provider(tree_rows(&elements, TreeForm::Edges), 1, &mut vec![]);
        let actual = execute(
            request,
            [left, right],
            &mut symbols,
            b"abcd",
            true,
            real_port(),
            Faults {
                left: left_fails.then_some(OperatingError::NotRun),
                right: right_fails.then_some(OperatingError::NotRead),
                ..Faults::default()
            },
        )
        .unwrap();
        assert!(
            actual.native_prepared,
            "the owner must compile actual native provenance"
        );
        if left_fails {
            assert_eq!(
                actual.attempts,
                [SourceAttempt::Refused(0, OperatingError::NotRun)]
            );
            assert_eq!(actual.reads, [1, 0]);
        } else if right_fails {
            let (last, before) = actual
                .attempts
                .split_last()
                .expect("RHS refusal is observed");
            assert_eq!(last, &SourceAttempt::Refused(1, OperatingError::NotRead));
            assert!(!before.is_empty());
            assert!(
                before
                    .iter()
                    .all(|attempt| *attempt == SourceAttempt::Available(0)),
                "every prior borrow is a successful LHS read; no source is retried after the refusal"
            );
            assert_eq!(actual.reads[1], 1);
        } else {
            assert_eq!(actual.attempts.first(), Some(&SourceAttempt::Available(0)));
            assert!(actual.attempts.contains(&SourceAttempt::Available(1)));
            assert!(
                actual
                    .attempts
                    .iter()
                    .all(|attempt| matches!(attempt, SourceAttempt::Available(_)))
            );
        }
        match error {
            Some(error) => assert_eq!(actual.result, Err(error)),
            None => assert!(actual.result.unwrap().cells.is_empty()),
        }
        assert!(actual.prefix.is_empty());
    }
}

#[test]
fn empty_native_step_join_validates_both_sources_without_sorting_rhs_keys() {
    use ryo_values::domain::plan_parts::types::{
        Comparison, LiteralText, Operand, QueryLiteral, Test,
    };
    for (refuse_right, foreign_tree) in [(false, false), (true, false), (false, true)] {
        let (left_elements, mut symbols) = wide_tree(1, true);
        let (right_elements, _) = wide_tree(1, true);
        let mut request = plan(Scenario::ProjectRetry);
        request.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxTree),
                    kept: vec![ColumnIndex(0)],
                    comparisons: vec![Comparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText(
                            "absent-kind".into(),
                        ))),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxChild),
                    kept: vec![ColumnIndex(1)],
                    comparisons: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: PlanInput::Step(StepId(0)),
                    right: PlanInput::Step(StepId(1)),
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
        request.emit.step = StepId(2);
        let (left, _) = provider(
            tree_rows(&left_elements, TreeForm::Elements),
            0,
            &mut vec![],
        );
        let (right, _) = provider(
            tree_rows(
                if foreign_tree {
                    &right_elements
                } else {
                    &left_elements
                },
                TreeForm::Edges,
            ),
            1,
            &mut vec![],
        );
        let actual = execute(
            request,
            [left, right],
            &mut symbols,
            b"abcd",
            false,
            real_port(),
            Faults {
                right: refuse_right.then_some(OperatingError::NotRead),
                ..Faults::default()
            },
        )
        .unwrap();
        assert!(
            actual.native_prepared,
            "the step-RHS native strategy must be real"
        );
        if refuse_right || foreign_tree {
            assert_eq!(actual.result, Err(OperatingError::NotRead));
        } else {
            let table = actual.result.unwrap();
            assert_eq!(table.count, RowCount(0));
            assert_eq!(table.kinds, [CellKind::Node]);
            assert_eq!(
                actual.work.native_positions, 1,
                "the admitted native empty publication was actually produced"
            );
        }
        assert!(
            actual.derived_builds.is_empty(),
            "including RawKeyOrder: no payload is needed for an empty primary"
        );
    }
}

#[test]
fn tree_lookup_preflight_preserves_required_support_and_optional_equality_fallback() {
    use ryo_derived::domain::contract::types::ViewRequest;
    use ryo_values::domain::plan_parts::types::{Comparison, Operand, Test};
    let (elements, mut symbols) = wide_tree(1, true);
    let right = tree_rows(&elements, TreeForm::Elements);
    let mut materialized = Table::default();
    materialize(&right, &mut materialized, &mut Vec::new());
    // The root's actual completed kind is a real provider symbol, not an invented id.
    let root_kind = materialized.cells[1];
    for empty in [true, false] {
        for delivery in [false, true] {
            for required in [0, 1, 2] {
                let left = if empty {
                    table_of(&[CellKind::Symbol], &[])
                } else {
                    table_of(&[CellKind::Symbol], &[&[root_kind]])
                };
                let filter = if required == 1 {
                    vec![Comparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Column(ColumnIndex(1)),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }]
                } else {
                    vec![]
                };
                let view: &[u32] = if required == 2 { &[0, 1, 2, 3, 4] } else { &[] };
                // Elements kind column1 is readable but not a native lookup key.
                let op = operation(&[0], &[1], &[(Side::Left, 0)], filter, view);
                let actual = invoke(
                    &op,
                    [Rows::Computed(&left), right.clone()],
                    &mut symbols,
                    b"abcd",
                    delivery,
                    real_port(),
                    Faults::default(),
                )
                .unwrap();
                assert_eq!(actual.native_prepared, false);
                if required != 0 {
                    assert!(
                        matches!(actual.result, Err(OperatingError::WrongOperation)),
                        "{:?}",
                        actual.result
                    );
                    assert!(actual.prefix.is_empty());
                    assert!(actual.derived_builds.is_empty());
                } else {
                    let result = actual.result.unwrap();
                    if !delivery {
                        assert_eq!(result.kinds, [CellKind::Symbol]);
                    }
                    assert_eq!(result.count.0, usize::from(!empty));
                    assert_eq!(result.cells, if empty { vec![] } else { vec![root_kind] });
                    if empty {
                        assert!(actual.derived_builds.is_empty());
                    } else {
                        assert!(
                            matches!(
                                actual.derived_builds.as_slice(),
                                [
                                    ViewRequest::NativeLookup { .. },
                                    ViewRequest::Equality { .. }
                                ]
                            ),
                            "{:?}",
                            actual.derived_builds
                        );
                    }
                }
            }
            // Supported native node keys must still accept a real filter, including empty LHS.
            let left = if empty {
                table_of(&[CellKind::Node], &[])
            } else {
                table_of(&[CellKind::Node], &[&[node(1)]])
            };
            let op = operation(
                &[0],
                &[0],
                &[(Side::Left, 0)],
                vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                &[],
            );
            let actual = invoke(
                &op,
                [Rows::Computed(&left), right.clone()],
                &mut symbols,
                b"abcd",
                delivery,
                real_port(),
                Faults::default(),
            )
            .unwrap();
            let result = actual.result.unwrap();
            if !delivery {
                assert_eq!(result.kinds, [CellKind::Node]);
            }
            assert_eq!(result.cells, if empty { vec![] } else { vec![node(1)] });
            assert_eq!(result.count.0, usize::from(!empty));
            if empty {
                assert!(actual.derived_builds.is_empty());
            } else {
                assert!(
                    actual
                        .derived_builds
                        .iter()
                        .any(|request| matches!(request, ViewRequest::NativeLookup { .. }))
                );
            }
        }
    }
}
