//! The real compiler chooses the selected-cut/Join/filter/Project alternative.
use super::capability::Witness;
use super::*;
use crate::domain::table_model::service::take_read_row_counts;
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
fn plan() -> Plan {
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
                    origin: None,
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

fn stored(relation: Relation, table: Table) -> Provider<'static> {
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
fn sources(symbols: &mut Symbols, count: u32, hit: bool) -> [Provider<'static>; 2] {
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
fn expected(count: u32) -> Vec<Cell> {
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
    take_read_row_counts();
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
    let candidate_reads = take_read_row_counts();
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
    take_read_row_counts();
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
    let baseline_reads = take_read_row_counts();
    assert!(
        !baseline.column_prepared,
        "observed Join cannot be replaced by a private column continuation"
    );
    assert_eq!(baseline.result.unwrap().cells, expected(64));
    assert_eq!(&baseline.frontiers[0], frontier);
    assert_eq!(ordinary_port.columns.load(Ordering::Relaxed), 0);
    assert!(ordinary_port.calls.load(Ordering::Relaxed) > 0);
    assert!(
        baseline_reads.0 > 0 && baseline_reads.1 > 0,
        "ordinary actual compiled route reads rows"
    );
    assert!(
        candidate_reads.0 * 5 <= baseline_reads.0,
        "80% fewer full-row reads: {candidate_reads:?} vs {baseline_reads:?}"
    );
    assert!(
        candidate_reads.1 * 5 <= baseline_reads.1,
        "80% fewer full-row cells: {candidate_reads:?} vs {baseline_reads:?}"
    );

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
