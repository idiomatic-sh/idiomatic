//! Ordered graph semantics and work counts through the compiled execution owner.
use super::super::*;
use crate::domain::operating::types::ExecutionObservation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostCell;
use ryo_values::domain::rows::service::push_row;
type Input = fn() -> Result<usize, OperatingError>;
type Value = usize;
type Failure = OperatingError;
const FROM: PlanInput = PlanInput::Relation(Relation::Function);
fn column(at: u32) -> ColumnIndex {
    ColumnIndex(at)
}
fn numbers(width: usize, rows: &[&[i64]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|n| Cell::Number(Integer(*n)))
                .collect::<Vec<_>>(),
        );
    }
    table
}
fn operation(root_keys: &[u32], edge_keys: &[u32]) -> Operation {
    Operation::OrderedDfs {
        from: FROM,
        edges: FROM,
        root: column(0),
        source: column(0),
        destination: column(1),
        order: column(2),
        edge: column(3),
        root_keys: root_keys.iter().copied().map(column).collect(),
        edge_keys: edge_keys.iter().copied().map(column).collect(),
    }
}
fn expected(out: &Table, width: usize, rows: &[&[i64]]) {
    let mut actual: Vec<_> = out
        .cells
        .chunks_exact(width)
        .map(<[Cell]>::to_vec)
        .collect();
    let mut wanted: Vec<Vec<_>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect()
        })
        .collect();
    actual.sort_unstable();
    wanted.sort_unstable();
    assert_eq!(out.kinds, vec![CellKind::Number; width]);
    assert_eq!(out.count.0, wanted.len());
    assert_eq!(out.cells.len(), out.count.0 * width);
    assert_eq!(actual, wanted);
}

fn host(table: &Table, index: usize) -> HostRelation {
    HostRelation {
        name: HostName(format!("dfs-input-{index}").into()),
        columns: table
            .kinds
            .iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("value-{at}").into()),
                kind: match kind {
                    CellKind::Number => HostKind::Number,
                    CellKind::Symbol => HostKind::Text,
                    other => panic!("unsupported scalar fixture kind {other:?}"),
                },
            })
            .collect(),
        rows: table
            .cells
            .chunks_exact(table.kinds.len())
            .map(|row| {
                HostRow(
                    row.iter()
                        .map(|cell| match cell {
                            Cell::Number(value) => HostCell::Number(*value),
                            other => panic!("unsupported scalar fixture cell {other:?}"),
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}
fn compute(
    operation: &Operation,
    roots: &Table,
    edges: &Table,
    observed: &mut ExecutionObservation,
) -> Result<Table, OperatingError> {
    let mut operation = operation.clone();
    let Operation::OrderedDfs {
        from,
        edges: input_edges,
        ..
    } = &mut operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    *from = PlanInput::Step(StepId(0));
    *input_edges = PlanInput::Step(StepId(1));
    let mut plan = plan(Scenario::ProjectRetry);
    plan.steps = [
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Hosted {
            relation: HostedNumber(1),
        },
        operation,
    ]
    .into_iter()
    .map(|operation| PlanStep {
        scope: StepScope::PerFile,
        operation,
    })
    .collect();
    plan.emit.step = StepId(2);
    execute_request(
        plan,
        &[host(roots, 0), host(edges, 1)],
        &[],
        &mut Symbols::default(),
        b"",
        false,
        observed,
    )
}
fn execute_request(
    plan: Plan,
    hosts: &[HostRelation],
    providers: &[(Relation, Table)],
    symbols: &mut Symbols,
    source: &[u8],
    delivery: bool,
    observed: &mut ExecutionObservation,
) -> Result<Table, OperatingError> {
    let target = plan.emit.step.0 as usize;
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("ordered.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: if delivery {
                DiagnosticDelivery::Forwarded
            } else {
                DiagnosticDelivery::Collected
            },
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(source),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )?;
    let read = |relation| {
        providers
            .iter()
            .find(|(kind, _)| *kind == relation)
            .map(|(_, table)| Rows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let result = if delivery {
        let mut table = Table::default();
        file.emit_rows(RuleIndex(0), &read, symbols, &mut |row| {
            table.cells.extend_from_slice(row);
            table.count.0 += 1;
            Ok(())
        })?;
        Ok(table)
    } else {
        let outcome = file.rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )?;
        match outcome.emit {
            RootOutcome::Complete(table) => Ok(table),
            RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
            _ => Err(OperatingError::NotRun),
        }
    };
    *observed = file.route_observation(RuleIndex(0), StepId(target as u32))?;
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(
        owner.derived.working_bytes(),
        0,
        "actual graph owner releases file resources"
    );
    result
}

fn numeric_order_and_diamond() -> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(
            4,
            &[
                &[1, 2, 20, 12],
                &[2, 4, 0, 24],
                &[1, 3, -10, 13],
                &[3, 4, 0, 34],
            ],
        ),
        &mut scratch,
    )?;
    expected(
        &out,
        5,
        &[
            &[1, 1, 3, 13, 0],
            &[1, 3, 4, 34, 1],
            &[1, 1, 2, 12, 2],
            &[1, 2, 4, 24, 3],
        ],
    );
    assert_eq!(scratch.dfs_expanded, 4);
    assert_eq!(scratch.dfs_examined, 4);
    Ok(out.count.0)
}
fn cycles_preserve_every_edge_visit() -> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(
            4,
            &[
                &[1, 1, 0, 11],
                &[1, 2, 1, 12],
                &[2, 1, 0, 21],
                &[2, 2, 1, 22],
            ],
        ),
        &mut scratch,
    )?;
    expected(
        &out,
        5,
        &[
            &[1, 1, 1, 11, 0],
            &[1, 1, 2, 12, 1],
            &[1, 2, 1, 21, 2],
            &[1, 2, 2, 22, 3],
        ],
    );
    assert_eq!(scratch.dfs_expanded, 2);
    assert_eq!(scratch.dfs_examined, 4);
    Ok(out.count.0)
}
fn parallel_distinct_orders_preserve_multiplicity() -> Result<usize, OperatingError> {
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 1, 12], &[1, 2, 0, 12]]),
        &mut ExecutionObservation::default(),
    )?;
    expected(&out, 5, &[&[1, 1, 2, 12, 0], &[1, 1, 2, 12, 1]]);
    Ok(out.count.0)
}
fn roots_are_independent_and_duplicates_are_deduplicated() -> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[2], &[1], &[1]]),
        &numbers(4, &[&[2, 3, 0, 23], &[1, 2, 0, 12]]),
        &mut scratch,
    )?;
    expected(
        &out,
        5,
        &[&[1, 1, 2, 12, 0], &[1, 2, 3, 23, 1], &[2, 2, 3, 23, 0]],
    );
    assert_eq!(scratch.dfs_expanded, 5);
    Ok(out.count.0)
}
fn partitions_and_multiple_keys_are_exact() -> Result<usize, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[2, 1], &[5, 4]),
        &numbers(
            3,
            &[&[1, 10, 100], &[1, 20, 100], &[1, 10, 200], &[1, 10, 100]],
        ),
        &numbers(
            6,
            &[
                &[1, 2, 0, 12, 10, 100],
                &[2, 3, 0, 23, 20, 100],
                &[1, 4, 0, 14, 20, 100],
                &[1, 5, 0, 15, 10, 200],
            ],
        ),
        &mut scratch,
    )?;
    expected(
        &out,
        7,
        &[
            &[100, 10, 1, 1, 2, 12, 0],
            &[100, 20, 1, 1, 4, 14, 0],
            &[200, 10, 1, 1, 5, 15, 0],
        ],
    );
    assert_eq!(scratch.dfs_examined, 3);
    Ok(out.count.0)
}
fn entry_node_encodes_initial_empty_class_seen() -> Result<usize, OperatingError> {
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[10]]),
        &numbers(
            4,
            &[
                &[10, 2, 0, 102],
                &[2, 1, 0, 21],
                &[1, 3, 0, 13],
                &[10, 3, 1, 103],
            ],
        ),
        &mut ExecutionObservation::default(),
    )?;
    expected(
        &out,
        5,
        &[
            &[10, 10, 2, 102, 0],
            &[10, 2, 1, 21, 1],
            &[10, 1, 3, 13, 2],
            &[10, 10, 3, 103, 3],
        ],
    );
    Ok(out.count.0)
}
fn ties_are_rejected_even_without_roots() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 3, 0, 13]]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
fn duplicate_edges_are_ties() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 2, 0, 12]]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
fn malformed_empty_input_is_rejected() -> Result<usize, OperatingError> {
    compute(
        &operation(&[8], &[4]),
        &numbers(1, &[]),
        &numbers(5, &[]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
fn empty_input_kind_mismatch_is_rejected() -> Result<usize, OperatingError> {
    let edges = Table {
        kinds: vec![
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number,
            CellKind::Number,
        ],
        ..Table::default()
    };
    compute(
        &operation(&[], &[]),
        &numbers(1, &[]),
        &edges,
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
fn deep_chain_and_work_are_bounded() -> Result<usize, OperatingError> {
    let n = 20_000;
    let mut edges = numbers(4, &[]);
    for i in 0..n {
        push_row(
            &mut edges,
            &[
                Cell::Number(Integer(i)),
                Cell::Number(Integer(i + 1)),
                Cell::Number(Integer(0)),
                Cell::Number(Integer(i)),
            ],
        );
    }
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[0]]),
        &edges,
        &mut scratch,
    )?;
    let expected_count = usize::try_from(n).expect("positive chain length");
    assert_eq!(out.count.0, expected_count);
    assert_eq!(scratch.dfs_examined, expected_count);
    assert_eq!(scratch.dfs_expanded, expected_count + 1);
    assert_eq!(scratch.dfs_frames_len, 0);
    Ok(out.count.0)
}
fn many_interleaved_partitions_bound_expansions() -> Result<usize, OperatingError> {
    let n = 1024;
    let mut roots = numbers(2, &[]);
    let mut edges = numbers(5, &[]);
    for partition in 0..n {
        push_row(
            &mut roots,
            &[Cell::Number(Integer(1)), Cell::Number(Integer(partition))],
        );
    }
    for [source, dest, order, id] in [[1, 2, 0, 12], [2, 4, 0, 24], [1, 3, 1, 13], [3, 4, 0, 34]] {
        for partition in 0..n {
            push_row(
                &mut edges,
                &[
                    Cell::Number(Integer(source)),
                    Cell::Number(Integer(dest)),
                    Cell::Number(Integer(order)),
                    Cell::Number(Integer(id)),
                    Cell::Number(Integer(partition)),
                ],
            );
        }
    }
    let mut scratch = ExecutionObservation::default();
    let out = compute(&operation(&[1], &[4]), &roots, &edges, &mut scratch)?;
    let expected_count = 4 * usize::try_from(n).expect("positive partition count");
    assert_eq!(out.count.0, expected_count);
    assert_eq!(scratch.dfs_examined, out.count.0);
    assert_eq!(scratch.dfs_expanded, expected_count);
    assert!(scratch.dfs_visited_len <= 4);
    Ok(out.count.0)
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("numeric_depth_first_order_and_diamond"),
        word: Word::Succeeds,
        input: || numeric_order_and_diamond,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("cycles_and_self_edges_are_emitted_before_seen_check"),
        word: Word::Succeeds,
        input: || cycles_preserve_every_edge_visit,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("parallel_distinct_orders_preserve_visits"),
        word: Word::Succeeds,
        input: || parallel_distinct_orders_preserve_multiplicity,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("duplicate_roots_and_independent_walks"),
        word: Word::Succeeds,
        input: || roots_are_independent_and_duplicates_are_deduplicated,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("multiple_exact_partition_keys_keep_output_order"),
        word: Word::Succeeds,
        input: || partitions_and_multiple_keys_are_exact,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("caller_entry_node_encodes_initial_empty_seen"),
        word: Word::Succeeds,
        input: || entry_node_encodes_initial_empty_class_seen,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("deep_chain_uses_iterative_frames"),
        word: Word::Succeeds,
        input: || deep_chain_and_work_are_bounded,
        expect: Expect::Value(20_000),
    },
    Case {
        name: CaseName("many_partitions_bound_work_without_timing"),
        word: Word::Succeeds,
        input: || many_interleaved_partitions_bound_expansions,
        expect: Expect::Value(4096),
    },
    Case {
        name: CaseName("ties_refused_even_with_empty_roots"),
        word: Word::Refuses,
        input: || ties_are_rejected_even_without_roots,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::DuplicateDfsOrder))
        }),
    },
    Case {
        name: CaseName("duplicate_edges_refused_as_order_ties"),
        word: Word::Refuses,
        input: || duplicate_edges_are_ties,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::DuplicateDfsOrder))
        }),
    },
    Case {
        name: CaseName("missing_columns_refused_on_empty_input"),
        word: Word::Refuses,
        input: || malformed_empty_input_is_rejected,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::Table(..)))),
    },
    Case {
        name: CaseName("entity_kinds_checked_on_empty_input"),
        word: Word::Refuses,
        input: || empty_input_kind_mismatch_is_rejected,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::ShapeMismatch))),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn hosted_plan(mut op: Operation, roots: u32, edges: u32) -> Plan {
    match &mut op {
        Operation::OrderedDfs { from, edges, .. } => {
            *from = PlanInput::Step(StepId(0));
            *edges = PlanInput::Step(StepId(1));
        }
        _ => panic!("DFS fixture operation"),
    }
    let mut request = plan(Scenario::ProjectRetry);
    request.steps = [
        Operation::Hosted {
            relation: HostedNumber(roots),
        },
        Operation::Hosted {
            relation: HostedNumber(edges),
        },
        op,
    ]
    .into_iter()
    .map(|operation| PlanStep {
        scope: StepScope::PerFile,
        operation,
    })
    .collect();
    request.emit.step = StepId(2);
    request
}
#[test]
fn file_partition_keys_keep_identical_node_values_separate() {
    use ryo_relations::domain::relations::types::NodeId;
    let n = |at| Cell::Node(NodeId(at));
    let edge_rows = [[1, 2, 10], [2, 3, 20], [1, 4, 20]].map(|[a, b, file]| {
        vec![
            n(a),
            n(b),
            Cell::Number(Integer(0)),
            Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            Cell::File(FileId(file as u32)),
        ]
    });
    let root_rows = [10, 20].map(|file| {
        vec![
            n(1),
            Cell::File(FileId(file)),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(0)),
        ]
    });
    let providers = [
        (
            Relation::SemanticChild,
            Table {
                kinds: kinds(Relation::SemanticChild).to_vec(),
                cells: edge_rows.into_iter().flatten().collect(),
                count: RowCount(3),
            },
        ),
        (
            Relation::SourceLocation,
            Table {
                kinds: kinds(Relation::SourceLocation).to_vec(),
                cells: root_rows.into_iter().flatten().collect(),
                count: RowCount(2),
            },
        ),
    ];
    let mut request = hosted_plan(operation(&[1], &[4]), 0, 1);
    request.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SourceLocation),
        comparisons: vec![],
        kept: vec![column(0), column(1)],
    };
    // The edge identity is a real projected node column, rather than a synthetic provider schema.
    request.steps[1].operation = Operation::Select {
        from: PlanInput::Relation(Relation::SemanticChild),
        comparisons: vec![],
        kept: vec![column(0), column(1), column(2), column(0), column(4)],
    };
    let out = execute_request(
        request,
        &[],
        &providers,
        &mut Symbols::default(),
        b"",
        false,
        &mut ExecutionObservation::default(),
    )
    .unwrap();
    assert_eq!(out.count, RowCount(2));
    assert_eq!(
        out.kinds,
        [
            CellKind::File,
            CellKind::Node,
            CellKind::Node,
            CellKind::Node,
            CellKind::Node,
            CellKind::Number
        ]
    );
    assert_eq!(
        out.cells,
        vec![
            Cell::File(FileId(10)),
            n(1),
            n(1),
            n(2),
            n(1),
            Cell::Number(Integer(0)),
            Cell::File(FileId(20)),
            n(1),
            n(1),
            n(4),
            n(1),
            Cell::Number(Integer(0))
        ]
    );
    assert!(!out.cells.chunks_exact(6).any(|row| row[3] == n(3)));
}
#[test]
fn row_delivery_and_collected_visits_keep_exact_order_and_duplicates() {
    let hosts = [
        host(&numbers(1, &[&[1], &[1]]), 0),
        host(
            &numbers(4, &[&[1, 2, 1, 12], &[1, 3, 0, 13], &[3, 1, 0, 31]]),
            1,
        ),
    ];
    let request = hosted_plan(operation(&[], &[]), 0, 1);
    let collected = execute_request(
        request.clone(),
        &hosts,
        &[],
        &mut Symbols::default(),
        b"",
        false,
        &mut ExecutionObservation::default(),
    )
    .unwrap();
    let emitted = execute_request(
        request,
        &hosts,
        &[],
        &mut Symbols::default(),
        b"",
        true,
        &mut ExecutionObservation::default(),
    )
    .unwrap();
    assert_eq!(collected.count, RowCount(3));
    assert_eq!(emitted.count, collected.count);
    assert_eq!(emitted.cells, collected.cells);
    assert_eq!(
        collected.cells,
        [1, 1, 3, 13, 0, 1, 3, 1, 31, 1, 1, 1, 2, 12, 2].map(|n| Cell::Number(Integer(n)))
    );
}
#[test]
fn text_vertices_and_partition_keys_use_the_actual_hosted_text_owner() {
    let text = |s: &str| HostCell::Text(s.into());
    let number = |n| HostCell::Number(Integer(n));
    let host_rows = |index, kinds: Vec<HostKind>, rows: Vec<Vec<HostCell>>| HostRelation {
        name: HostName(format!("dfs-text-{index}").into()),
        columns: kinds
            .into_iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("c{at}").into()),
                kind,
            })
            .collect(),
        rows: rows.into_iter().map(HostRow).collect(),
    };
    let hosts = [
        host_rows(
            0,
            vec![HostKind::Text; 2],
            vec![
                vec![text("a"), text("ctx")],
                vec![text("a"), text("ctx")],
                vec![text("b"), text("ctx")],
            ],
        ),
        host_rows(
            1,
            vec![
                HostKind::Text,
                HostKind::Text,
                HostKind::Number,
                HostKind::Text,
                HostKind::Text,
            ],
            vec![
                vec![text("a"), text("b"), number(0), text("edge"), text("ctx")],
                vec![text("b"), text("c"), number(0), text("edge"), text("ctx")],
                vec![text("b"), text("a"), number(1), text("edge"), text("ctx")],
            ],
        ),
    ];
    let mut symbols = Symbols::default();
    let mut observed = ExecutionObservation::default();
    let out = execute_request(
        hosted_plan(operation(&[1], &[4]), 0, 1),
        &hosts,
        &[],
        &mut symbols,
        b"",
        false,
        &mut observed,
    )
    .unwrap();
    assert_eq!(out.count, RowCount(6));
    assert_eq!(
        out.kinds,
        [
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number
        ]
    );
    let texts = crate::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(b""),
        paths: &[],
        arenas: &[],
    };
    let mut rows = Vec::new();
    for row in out.cells.chunks_exact(6) {
        let strings = row[..5]
            .iter()
            .map(|cell| {
                let crate::domain::table_texts::types::Datum::Text(text) =
                    crate::domain::table_texts::service::datum(*cell, &texts)
                else {
                    panic!("owned text")
                };
                text.0.to_owned()
            })
            .collect::<Vec<_>>();
        let Cell::Number(ordinal) = row[5] else {
            panic!("ordinal")
        };
        rows.push((strings, ordinal.0));
    }
    assert_eq!(
        rows,
        vec![
            (
                vec!["ctx", "a", "a", "b", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                0
            ),
            (
                vec!["ctx", "a", "b", "c", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                1
            ),
            (
                vec!["ctx", "a", "b", "a", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                2
            ),
            (
                vec!["ctx", "b", "b", "c", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                0
            ),
            (
                vec!["ctx", "b", "b", "a", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                1
            ),
            (
                vec!["ctx", "b", "a", "b", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                2
            )
        ]
    );
    assert_eq!(observed.dfs_expanded, 6);
    assert_eq!(observed.dfs_examined, 6);
}
#[test]
fn one_file_owner_resets_dfs_across_closure_empty_and_different_graph() {
    let hosts = [
        host(&numbers(1, &[&[1]]), 0),
        host(&numbers(4, &[&[1, 2, 0, 12], &[2, 3, 0, 23]]), 1),
        host(&numbers(1, &[]), 2),
        host(&numbers(4, &[]), 3),
        host(&numbers(1, &[&[9]]), 4),
        host(&numbers(4, &[&[9, 8, 0, 98]]), 5),
    ];
    let first = hosted_plan(operation(&[], &[]), 0, 1);
    let mut closure = first.clone();
    closure.steps[2].operation = Operation::Closure {
        edges: PlanInput::Step(StepId(1)),
        keys: vec![],
        source: column(0),
        destination: column(1),
    };
    let plans = vec![
        first,
        closure,
        hosted_plan(operation(&[], &[]), 2, 3),
        hosted_plan(operation(&[], &[]), 4, 5),
    ];
    let rules = (0..4)
        .map(|index| {
            let mut value = rule();
            value.id = RuleId(format!("dfs-lifecycle-{index}").into());
            value
        })
        .collect::<Vec<_>>();
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans,
            bindings: vec![vec![]; 4],
            observations: vec![vec![]; 4],
            schedule: RuleSchedule::Independent,
        },
        &rules,
        &[Digest([0; 16]); 4],
        &hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("dfs-lifecycle.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 4],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut outputs = Vec::new();
    for index in 0..4 {
        let RootOutcome::Complete(table) = file
            .rule(
                RuleIndex(index),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                &read,
                &mut symbols,
            )
            .unwrap()
            .emit
        else {
            panic!("complete independent rule")
        };
        let state = file.route_observation(RuleIndex(index), StepId(2)).unwrap();
        if index == 0 {
            assert!(state.dfs_adjacency_len > 0);
        }
        if index == 2 {
            assert_eq!(state.dfs_adjacency_len, 0);
            assert_eq!(state.dfs_frames_len, 0);
            assert_eq!(state.dfs_visited_len, 0);
        }
        outputs.push(table);
    }
    expected(&outputs[0], 5, &[&[1, 1, 2, 12, 0], &[1, 2, 3, 23, 1]]);
    assert_eq!(outputs[1].count, RowCount(3));
    assert_eq!(outputs[2].count, RowCount(0));
    expected(&outputs[3], 5, &[&[9, 9, 8, 98, 0]]);
    let owner = Rc::clone(&file.owner);
    file.finish().unwrap();
    assert_eq!(owner.derived.working_bytes(), 0);
}
