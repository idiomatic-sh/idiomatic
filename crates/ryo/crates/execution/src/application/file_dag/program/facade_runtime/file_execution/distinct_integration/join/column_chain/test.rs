//! The real compiler chooses the selected-cut/Join/filter/Project alternative.
use super::super::capability::test::Witness;
use super::super::*;
use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{
    Comparison, Input, LiteralText, Operand, QueryLiteral, Test,
};
use std::sync::atomic::Ordering;
fn column(number: u32) -> ColumnIndex {
    ColumnIndex(number)
}

/// The same compiled family observed at PYI034 cut 116 and PLC0206 cut 233.
pub(in super::super) fn plan() -> Plan {
    let steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Relation(Relation::Function),
                columns: vec![column(0), column(2)],
                distinct: RepeatedRows::Dropped,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::SemanticChild),
                left_on: vec![column(0), column(1)],
                right_on: vec![column(0), column(4)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: column(0),
                    },
                    JoinColumn {
                        side: Side::Left,
                        column: column(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: column(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: column(3),
                    },
                ],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Step(StepId(1)),
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(column(3)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("body".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(2)),
                columns: vec![column(0), column(1), column(2)],
                distinct: RepeatedRows::Dropped,
            },
        },
    ];
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(3),
            subject: column(0),
            key: vec![column(0)],
            headings: Vec::new(),
        },
        // The shared cut is also handed outward, exactly as a real kept cut is.
        frontier: vec![Input::Step(StepId(0))],
    }
}

pub(in super::super) fn stored(relation: Relation, table: Table) -> Provider<'static> {
    let columns = (0..table.kinds.len())
        .map(|column| {
            FactColumn(
                table
                    .cells
                    .chunks_exact(table.kinds.len())
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect();
    Provider {
        relation,
        table,
        columns,
        compact: None,
        tree: None,
        representation: RepresentationKind::Stored,
    }
}
pub(in super::super) fn sources(
    symbols: &mut Symbols,
    count: u32,
    hit: bool,
) -> [Provider<'static>; 2] {
    let body = intern_derived(symbols, ByteSlice(b""), NameText("body"));
    let other = intern_derived(symbols, ByteSlice(b""), NameText("other"));
    let file = FileId(0);
    let left = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(count as usize),
        cells: (0..count)
            .flat_map(|row| {
                [
                    Cell::Node(node_id(file, Position(row))),
                    Cell::Symbol(other),
                    Cell::File(file),
                    Cell::Span(Span {
                        start: Offset(row),
                        end: Offset(row + 1),
                    }),
                ]
            })
            .collect(),
    };
    let right = Table {
        kinds: kinds(Relation::SemanticChild).to_vec(),
        count: RowCount((count * 10) as usize),
        cells: (0..count)
            .flat_map(|row| {
                (0..10).flat_map(move |repeated| {
                    [
                        Cell::Node(node_id(file, Position(row))),
                        Cell::Node(node_id(file, Position(row + 1000))),
                        Cell::Number(Integer(repeated)),
                        Cell::Symbol(if hit && repeated != 9 { body } else { other }),
                        Cell::File(file),
                    ]
                })
            })
            .collect(),
    };
    [
        stored(Relation::Function, left),
        stored(Relation::SemanticChild, right),
    ]
}
pub(in super::super) fn expected(count: u32) -> Vec<Cell> {
    (0..count)
        .flat_map(|row| {
            [
                Cell::Node(node_id(FileId(0), Position(row))),
                Cell::File(FileId(0)),
                Cell::Node(node_id(FileId(0), Position(row + 1000))),
            ]
        })
        .collect()
}
#[test]
fn selected_cut_join_filter_project_keeps_exact_rows_frontier_and_required_capability() {
    let mut symbols = Symbols::default();
    let providers = sources(&mut symbols, 64, true);
    let port = Arc::new(Witness::default());
    let candidate = execute(
        plan(),
        providers,
        &mut symbols,
        b"",
        false,
        port.clone(),
        Faults::default(),
    )
    .unwrap();
    assert!(
        candidate.column_prepared,
        "real compiler must install this exact column chain"
    );
    assert_eq!(candidate.result.unwrap().cells, expected(64));
    assert!(
        port.columns.load(Ordering::Relaxed) > 0,
        "actual column-cell semantic call, not just prepared metadata"
    );
    let frontier = &candidate.frontiers[0];
    assert_eq!(frontier.count, RowCount(64));
    assert_eq!(
        frontier.cells,
        (0..64)
            .flat_map(|at| [
                Cell::Node(node_id(FileId(0), Position(at))),
                Cell::File(FileId(0))
            ])
            .collect::<Vec<_>>()
    );
    // An authored additional reader is the genuine pre-work decline control.
    let mut ordinary = plan();
    ordinary.frontier.push(PlanInput::Step(StepId(1)));
    let providers = sources(&mut symbols, 64, true);
    let ordinary_port = Arc::new(Witness::default());
    let baseline = execute(
        ordinary,
        providers,
        &mut symbols,
        b"",
        false,
        ordinary_port.clone(),
        Faults::default(),
    )
    .unwrap();
    assert!(
        !baseline.column_prepared,
        "observed Join cannot be replaced by a private column continuation"
    );
    assert_eq!(baseline.result.unwrap().cells, expected(64));
    assert_eq!(&baseline.frontiers[0], frontier);
    assert_eq!(ordinary_port.columns.load(Ordering::Relaxed), 0);
    assert!(ordinary_port.calls.load(Ordering::Relaxed) > 0);
    // This extra consumer changes observable publication work. It proves a real
    // decline and output parity, not a matched-workload percentage reduction.

    let providers = sources(&mut symbols, 64, true);
    let refusing = Arc::new(Witness {
        refuse: true,
        only_columns: true,
        ..Witness::default()
    });
    let result = execute(
        plan(),
        providers,
        &mut symbols,
        b"",
        false,
        refusing.clone(),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(
        refusing.columns.load(Ordering::Relaxed),
        1,
        "refusal before first output, without optional fallback"
    );
    assert!(matches!(
        result.result,
        Err(OperatingError::Table(
            ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: 60001 }
        ))
    ));
}
#[test]
fn no_hits_missing_provider_and_partial_empty_sources_keep_refusal_and_frontier_boundaries() {
    let mut symbols = Symbols::default();
    let no_hits = execute(
        plan(),
        sources(&mut symbols, 1, false),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert!(no_hits.result.unwrap().cells.is_empty());
    assert_eq!(no_hits.frontiers[0].count, RowCount(1));
    let absent = execute(
        plan(),
        sources(&mut symbols, 1, true),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults {
            left: Some(OperatingError::NotRead),
            ..Faults::default()
        },
    );
    let absent = absent.unwrap();
    assert!(matches!(absent.result, Err(OperatingError::NotRead)));
    assert!(matches!(
        absent.frontier_errors.as_slice(),
        [OperatingError::NotRead]
    ));
    let partial = execute(
        plan(),
        sources(&mut symbols, 0, true),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults {
            partials: vec![crate::domain::coverage::types::Partial {
                relation: Relation::SemanticChild,
                cause: ryo_values::domain::stored_facts::types::PartialCause("incomplete".into()),
                lacking: crate::domain::coverage::types::Lacking::Any,
                at: None,
            }],
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(partial.result.unwrap().cells.is_empty());
    assert!(partial.frontiers[0].cells.is_empty());
}

#[test]
fn ragged_semantic_child_normalizes_its_complete_prefix_before_column_chain_fallback() {
    let mut symbols = Symbols::default();
    let body = intern_derived(&mut symbols, ByteSlice(b""), NameText("body"));
    let other = intern_derived(&mut symbols, ByteSlice(b""), NameText("other"));
    let parent = Cell::Node(node_id(FileId(0), Position(1)));
    let child = Cell::Node(node_id(FileId(0), Position(2)));
    let function = stored(
        Relation::Function,
        Table {
            kinds: kinds(Relation::Function).to_vec(),
            count: RowCount(1),
            cells: vec![
                parent,
                Cell::Symbol(other),
                Cell::File(FileId(0)),
                Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(1),
                }),
            ],
        },
    );
    let mut right = stored(
        Relation::SemanticChild,
        Table {
            kinds: kinds(Relation::SemanticChild).to_vec(),
            count: RowCount(2),
            cells: [0, 1]
                .into_iter()
                .flat_map(|ordinal| {
                    [
                        parent,
                        child,
                        Cell::Number(Integer(ordinal)),
                        Cell::Symbol(body),
                        Cell::File(FileId(0)),
                    ]
                })
                .collect(),
        },
    );
    // Keep the provider physically ragged. No fixture decoder, truncation of
    // other columns or rebuilt logical table may perform owner normalization.
    right.columns[2].0.truncate(1);
    assert_eq!(
        right
            .columns
            .iter()
            .map(|column| column.0.len())
            .collect::<Vec<_>>(),
        [2, 2, 1, 2, 2]
    );
    let witness = Arc::new(Witness::default());
    let actual = execute(
        plan(),
        [function, right],
        &mut symbols,
        b"",
        false,
        witness.clone(),
        Faults::default(),
    )
    .unwrap();
    assert!(
        actual.column_prepared,
        "the original authored chain remains a real compiled candidate"
    );
    assert_eq!(
        witness.columns.load(Ordering::Relaxed),
        0,
        "owner-normalized computed RHS must decline the stored column-cell alternative"
    );
    assert!(
        witness.calls.load(Ordering::Relaxed) > 0,
        "the actual registered Join handles the normalized population"
    );
    let table = actual.result.unwrap();
    assert_eq!(table.count, RowCount(1));
    assert_eq!(table.cells, [parent, Cell::File(FileId(0)), child]);
    assert!(actual.frontier_errors.is_empty());
    assert_eq!(actual.frontiers.len(), 1);
    assert_eq!(
        actual.frontiers[0],
        Table {
            kinds: vec![CellKind::Node, CellKind::File],
            count: RowCount(1),
            cells: vec![parent, Cell::File(FileId(0))],
        }
    );
}

#[test]
fn actual_column_join_counts_matching_visits_not_population_size_even_on_refusal() {
    use ryo_operators::inbound::use_cases::operators::interface::{
        JoinInput, JoinRow, OperatorAnswer, OperatorCall, OperatorsUseCase, PortableOperatorFault,
    };
    #[derive(Debug)]
    struct AfterFirstMatch {
        witness: Arc<Witness>,
        refuse: bool,
    }
    impl OperatorsUseCase for AfterFirstMatch {
        fn operators(
            &self,
            call: OperatorCall<'_>,
        ) -> Result<OperatorAnswer, PortableOperatorFault> {
            match call {
                OperatorCall::EqualityJoin {
                    input:
                        JoinInput::Values {
                            left: JoinRow::Columns(left),
                            left_on,
                            driver,
                            ..
                        },
                } if self.refuse => {
                    self.witness.calls.fetch_add(1, Ordering::Relaxed);
                    self.witness.columns.fetch_add(1, Ordering::Relaxed);
                    let key = left_on
                        .iter()
                        .map(|column| {
                            left.cell(*column)
                                .expect("the actual prepared left key exists")
                        })
                        .collect::<Vec<_>>();
                    assert!(
                        driver
                            .next(&key)
                            .expect("the actual indexed candidate is readable")
                    );
                    Err(PortableOperatorFault::ShapeMismatch)
                }
                call => self.witness.operators(call),
            }
        }
    }
    for (key, refuse_after_match, hits) in [(1, false, 4), (99, false, 0), (1, true, 1)] {
        let mut symbols = Symbols::default();
        let name = intern_derived(&mut symbols, ByteSlice(b""), NameText("function"));
        let body = intern_derived(&mut symbols, ByteSlice(b""), NameText("body"));
        let function = stored(
            Relation::Function,
            Table {
                kinds: kinds(Relation::Function).to_vec(),
                count: RowCount(3),
                cells: [key, key, 9]
                    .into_iter()
                    .flat_map(|node| {
                        [
                            Cell::Node(NodeId(node)),
                            Cell::Symbol(name),
                            Cell::File(FileId(0)),
                            Cell::Span(Span {
                                start: Offset(0),
                                end: Offset(1),
                            }),
                        ]
                    })
                    .collect(),
            },
        );
        let right = stored(
            Relation::SemanticChild,
            Table {
                kinds: kinds(Relation::SemanticChild).to_vec(),
                count: RowCount(5),
                cells: [(1, 10), (1, 11), (8, 12), (10, 13), (11, 14)]
                    .into_iter()
                    .enumerate()
                    .flat_map(|(ordinal, (parent, child))| {
                        [
                            Cell::Node(NodeId(parent)),
                            Cell::Node(NodeId(child)),
                            Cell::Number(Integer(ordinal as i64)),
                            Cell::Symbol(body),
                            Cell::File(FileId(0)),
                        ]
                    })
                    .collect(),
            },
        );
        let mut authored = plan();
        let Operation::Project { distinct, .. } = &mut authored.steps[0].operation else {
            unreachable!()
        };
        *distinct = RepeatedRows::Kept;
        let witness = Arc::new(Witness::default());
        let actual = execute(
            authored,
            [function, right],
            &mut symbols,
            b"",
            false,
            Arc::new(AfterFirstMatch {
                witness: witness.clone(),
                refuse: refuse_after_match,
            }),
            Faults::default(),
        )
        .unwrap();
        assert!(
            actual.column_prepared,
            "this proof requires the actual compiled column alternative"
        );
        assert_eq!(
            witness.columns.load(Ordering::Relaxed),
            if refuse_after_match { 1 } else { 3 },
            "the real column capability is called once per left row, stopping on its first refusal"
        );
        assert_eq!(
            actual.fed[1].0[1], hits,
            "count every actual next_match result, including the match before a semantic failure"
        );
        if refuse_after_match {
            assert!(matches!(actual.result, Err(OperatingError::ShapeMismatch)));
        } else {
            let expected = if key == 99 {
                vec![]
            } else {
                [10, 11]
                    .into_iter()
                    .flat_map(|child| {
                        [
                            Cell::Node(NodeId(1)),
                            Cell::File(FileId(0)),
                            Cell::Node(NodeId(child)),
                        ]
                    })
                    .collect()
            };
            assert_eq!(actual.result.unwrap().cells, expected);
        }
    }
}

#[test]
fn empty_column_primary_admits_rhs_and_metadata_without_building_an_index() {
    for bad_rhs in [
        None,
        Some(OperatingError::NotRead),
        Some(OperatingError::ShapeMismatch),
    ] {
        let mut symbols = Symbols::default();
        let [mut left, mut right] = sources(&mut symbols, 1, true);
        for column in &mut left.columns {
            column.0.clear();
        }
        let faults = if bad_rhs == Some(OperatingError::ShapeMismatch) {
            right.columns.pop(); // Actual malformed metadata, not a fabricated refusal.
            Faults::default()
        } else {
            Faults {
                right: bad_rhs.clone(),
                ..Faults::default()
            }
        };
        let witness = Arc::new(Witness::default());
        let actual = execute(
            plan(),
            [left, right],
            &mut symbols,
            b"",
            false,
            witness.clone(),
            faults,
        )
        .unwrap();
        assert!(
            actual.column_prepared,
            "the real compiled column strategy is installed"
        );
        if let Some(error) = bad_rhs {
            assert_eq!(actual.result, Err(error));
        } else {
            let table = actual.result.unwrap();
            assert_eq!(table.count, RowCount(0));
            assert_eq!(
                table.kinds,
                [CellKind::Node, CellKind::File, CellKind::Node]
            );
        }
        assert_eq!(witness.columns.load(Ordering::Relaxed), 0);
        assert!(
            actual.derived_builds.is_empty(),
            "no equality, graph or other payload is built for an empty primary"
        );
        assert!(actual.attempts.iter().any(|attempt| matches!(
            attempt,
            SourceAttempt::Available(1) | SourceAttempt::Refused(1, _)
        )));
    }
}
