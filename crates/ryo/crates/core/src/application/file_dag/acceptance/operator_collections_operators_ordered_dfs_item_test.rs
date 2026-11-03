//! Ordered DFS policy-independent graph cases and deterministic work guards.
use crate::application::file_dag::feed::Feed;
use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::collections::operators::ordered_dfs::item::OrderedDfsItem;
use crate::domain::table_model::{
    service::push_row,
    types::{Rows, Table},
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::{
    relations::types::{CellKind, Integer, Relation, Value as Cell},
    symbols::types::Symbols,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as Source},
    plans::types::{Operation, QueryOperator},
};
type Input = fn() -> Result<usize, OperatingError>;
type Value = usize;
type Failure = OperatingError;
const FROM: Source = Source::Relation(Relation::Function);
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
fn compute(
    op: &Operation,
    roots: &Table,
    edges: &Table,
    scratch: &mut OperatorScratch,
) -> Result<Table, OperatingError> {
    let mut symbols = Symbols::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch,
    };
    OrderedDfsItem.compute(
        op,
        &[Rows::Computed(roots), Rows::Computed(edges)],
        &mut context,
        &mut out,
    )?;
    Ok(out)
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
    assert_eq!(actual, wanted);
}
fn numeric_order_and_diamond() -> Result<usize, OperatingError> {
    let mut scratch = OperatorScratch::default();
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
    assert_eq!(scratch.observation.dfs_expanded, 4);
    assert_eq!(scratch.observation.dfs_examined, 4);
    Ok(out.count.0)
}
fn cycles_preserve_every_edge_visit() -> Result<usize, OperatingError> {
    let mut scratch = OperatorScratch::default();
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
    assert_eq!(scratch.observation.dfs_expanded, 2);
    assert_eq!(scratch.observation.dfs_examined, 4);
    Ok(out.count.0)
}
fn parallel_distinct_orders_preserve_multiplicity() -> Result<usize, OperatingError> {
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 1, 12], &[1, 2, 0, 12]]),
        &mut OperatorScratch::default(),
    )?;
    expected(&out, 5, &[&[1, 1, 2, 12, 0], &[1, 1, 2, 12, 1]]);
    Ok(out.count.0)
}
fn roots_are_independent_and_duplicates_are_deduplicated() -> Result<usize, OperatingError> {
    let mut scratch = OperatorScratch::default();
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
    assert_eq!(scratch.observation.dfs_expanded, 5);
    Ok(out.count.0)
}
fn partitions_and_multiple_keys_are_exact() -> Result<usize, OperatingError> {
    let mut scratch = OperatorScratch::default();
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
    assert_eq!(scratch.observation.dfs_examined, 3);
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
        &mut OperatorScratch::default(),
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
        &mut OperatorScratch::default(),
    )
    .map(|out| out.count.0)
}
fn duplicate_edges_are_ties() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 2, 0, 12]]),
        &mut OperatorScratch::default(),
    )
    .map(|out| out.count.0)
}
fn malformed_empty_input_is_rejected() -> Result<usize, OperatingError> {
    compute(
        &operation(&[8], &[4]),
        &numbers(1, &[]),
        &numbers(5, &[]),
        &mut OperatorScratch::default(),
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
        &mut OperatorScratch::default(),
    )
    .map(|out| out.count.0)
}
fn nonnumeric_order_cell_is_rejected() -> Result<usize, OperatingError> {
    let mut edges = numbers(4, &[&[1, 2, 0, 12]]);
    edges.cells[2] = Cell::Flag(ryo_language::domain::relations::types::Flag::Raised);
    compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &edges,
        &mut OperatorScratch::default(),
    )
    .map(|out| out.count.0)
}
fn scratch_reuse_across_graphs_empty_and_old_operator() -> Result<usize, OperatingError> {
    let mut scratch = OperatorScratch::default();
    let roots = numbers(1, &[&[1]]);
    let edges = numbers(4, &[&[1, 2, 0, 12], &[2, 3, 0, 23]]);
    let first = compute(&operation(&[], &[]), &roots, &edges, &mut scratch)?;
    assert_eq!(first.count.0, 2);
    assert!(scratch.observation.dfs_adjacency_capacity > 0);
    assert!(scratch.observation.local_resources_released);
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut scratch,
    };
    crate::domain::operator::collections::operators::closure::item::ClosureItem.compute(
        &Operation::Closure {
            edges: FROM,
            keys: vec![],
            source: column(0),
            destination: column(1),
        },
        &[Rows::Computed(&edges)],
        &mut context,
        &mut Table::default(),
    )?;
    let empty = compute(
        &operation(&[], &[]),
        &numbers(1, &[]),
        &numbers(4, &[]),
        &mut scratch,
    )?;
    assert_eq!(empty.count.0, 0);
    assert_eq!(scratch.observation.dfs_visited_len, 0);
    assert_eq!(scratch.observation.dfs_frames_len, 0);
    assert_eq!(scratch.observation.dfs_adjacency_len, 0);
    assert_eq!(scratch.observation.dfs_adjacency_capacity, 0);
    assert!(scratch.observation.local_resources_released);
    let next = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[9]]),
        &numbers(4, &[&[9, 8, 0, 98]]),
        &mut scratch,
    )?;
    expected(&next, 5, &[&[9, 9, 8, 98, 0]]);
    Ok(next.count.0)
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
    let mut scratch = OperatorScratch::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[0]]),
        &edges,
        &mut scratch,
    )?;
    let expected_count = usize::try_from(n).expect("positive chain length");
    assert_eq!(out.count.0, expected_count);
    assert_eq!(scratch.observation.dfs_examined, expected_count);
    assert_eq!(scratch.observation.dfs_expanded, expected_count + 1);
    assert_eq!(scratch.observation.dfs_frames_len, 0);
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
    let mut scratch = OperatorScratch::default();
    let out = compute(&operation(&[1], &[4]), &roots, &edges, &mut scratch)?;
    let expected_count = 4 * usize::try_from(n).expect("positive partition count");
    assert_eq!(out.count.0, expected_count);
    assert_eq!(scratch.observation.dfs_examined, out.count.0);
    assert_eq!(scratch.observation.dfs_expanded, expected_count);
    assert!(scratch.observation.dfs_visited_len <= 4);
    Ok(out.count.0)
}
fn ordinal_overflow_is_checked_after_the_last_representable_visit() -> Result<usize, OperatingError>
{
    use crate::domain::table_model::types::KeyIndex;
    use crate::domain::table_reach::{
        service::{begin_ordered, next_ordered, prepare_ordered},
        types::{Edges, OrderedGraph, OrderedWalk},
    };
    let table = numbers(4, &[&[1, 2, 0, 12], &[1, 3, 1, 13]]);
    let mut graph = OrderedGraph {
        edge_columns: vec![column(0), column(1)],
        ..OrderedGraph::default()
    };
    let mut order = KeyIndex::default();
    prepare_ordered(&table, column(2), &mut order, &mut graph)?;
    let source = [column(0)];
    let edges = Edges {
        table: &table,
        order: &order,
        from: &source,
        to: column(1),
    };
    let mut walk = OrderedWalk::default();
    begin_ordered(&edges, &graph, &[], Cell::Number(Integer(1)), &mut walk);
    walk.ordinal = Some(i64::MAX);
    let first = next_ordered(&edges, &graph, &[], &mut walk)?.unwrap();
    assert_eq!(first.1, Integer(i64::MAX));
    assert!(!walk.visited.contains(&Cell::Number(Integer(2))));
    next_ordered(&edges, &graph, &[], &mut walk).map(|_| 0)
}
fn retained_text_fixture(source: &[u8]) -> (Symbols, Table, Table) {
    use ryo_language::domain::{
        relations::types::{
            FileId, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash, WorkerNumber,
        },
        symbols::service::{intern, places_mut},
        symbols::types::ByteSlice,
    };
    let mut symbols = Symbols::default();
    let left = Cell::Symbol(intern(
        &mut symbols,
        ByteSlice(source),
        Offset(0),
        Offset(1),
    ));
    let right = Cell::Symbol(intern(
        &mut symbols,
        ByteSlice(source),
        Offset(1),
        Offset(2),
    ));
    let ctx = Cell::Symbol(intern(
        &mut symbols,
        ByteSlice(source),
        Offset(3),
        Offset(6),
    ));
    *places_mut(&mut symbols) = vec![(0, 1), (1, 2), (2, 3), (3, 6), (6, 10)]
        .into_iter()
        .map(|(start, end)| NamePlace {
            origin: Origin::Derived(WorkerNumber(0)),
            file: FileId(0),
            start: Offset(start),
            end: Offset(end),
        })
        .collect();
    // Deliberately equal hashes: actual retained text, not a hash, is the lookup identity.
    let retained = |place| {
        Cell::Name(Name {
            hash: TextHash(99),
            place: PlaceNumber(place),
        })
    };
    let mut roots = Table {
        kinds: vec![CellKind::Symbol; 2],
        ..Table::default()
    };
    push_row(&mut roots, &[left, ctx]);
    push_row(&mut roots, &[retained(0), retained(3)]);
    push_row(&mut roots, &[retained(1), ctx]);
    let mut edges = Table {
        kinds: vec![
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number,
            CellKind::Symbol,
            CellKind::Symbol,
        ],
        ..Table::default()
    };
    push_row(
        &mut edges,
        &[
            retained(0),
            right,
            Cell::Number(Integer(0)),
            retained(4),
            retained(3),
        ],
    );
    push_row(
        &mut edges,
        &[
            right,
            retained(2),
            Cell::Number(Integer(0)),
            retained(4),
            ctx,
        ],
    );
    push_row(
        &mut edges,
        &[
            retained(1),
            retained(0),
            Cell::Number(Integer(1)),
            retained(4),
            ctx,
        ],
    );
    (symbols, roots, edges)
}

fn retained_text_keys_and_vertices_preserve_output_cells() -> Result<usize, OperatingError> {
    use ryo_language::domain::relations::types::{Name, PlaceNumber};
    use ryo_values::domain::stored_facts::types::ArenaBytes;
    let source = b"abcctxedge";
    let arenas = vec![ArenaBytes(source.to_vec())];
    let (mut symbols, roots, edges) = retained_text_fixture(source);
    let mut scratch = OperatorScratch::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &arenas,
        memo: &mut Vec::new(),
        scratch: &mut scratch,
    };
    OrderedDfsItem.compute(
        &operation(&[1], &[4]),
        &[Rows::Computed(&roots), Rows::Computed(&edges)],
        &mut context,
        &mut out,
    )?;
    assert_eq!(out.count.0, 6);
    assert_eq!(
        out.kinds,
        vec![
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number
        ]
    );
    assert!(out.cells.chunks_exact(6).any(|row| matches!(
        row[1],
        Cell::Name(Name {
            place: PlaceNumber(1),
            ..
        })
    )));
    for row in out.cells.chunks_exact(6) {
        assert!(matches!(
            row[4],
            Cell::Name(Name {
                place: PlaceNumber(4),
                ..
            })
        ));
    }
    assert_eq!(scratch.observation.dfs_expanded, 6);
    assert_eq!(scratch.observation.dfs_examined, 6);
    Ok(out.count.0)
}
fn file_keys_keep_reused_nodes_separate() -> Result<usize, OperatingError> {
    use ryo_language::domain::relations::types::{FileId, NodeId};
    let mut roots = Table {
        kinds: vec![CellKind::Node, CellKind::File],
        ..Table::default()
    };
    for file in [10, 20] {
        push_row(
            &mut roots,
            &[Cell::Node(NodeId(1)), Cell::File(FileId(file))],
        );
    }
    let mut edges = Table {
        kinds: vec![
            CellKind::Node,
            CellKind::Node,
            CellKind::Number,
            CellKind::Node,
            CellKind::File,
        ],
        ..Table::default()
    };
    for [source, dest, id, file] in [[1, 2, 12, 10], [2, 3, 23, 20], [1, 4, 14, 20]] {
        push_row(
            &mut edges,
            &[
                Cell::Node(NodeId(source)),
                Cell::Node(NodeId(dest)),
                Cell::Number(Integer(0)),
                Cell::Node(NodeId(id)),
                Cell::File(FileId(
                    u32::try_from(file).expect("small fixture file number"),
                )),
            ],
        );
    }
    let out = compute(
        &operation(&[1], &[4]),
        &roots,
        &edges,
        &mut OperatorScratch::default(),
    )?;
    assert_eq!(out.count.0, 2);
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
    assert!(
        !out.cells
            .chunks_exact(6)
            .any(|row| row[3] == Cell::Node(NodeId(3)))
    );
    Ok(out.count.0)
}

struct Held<'f> {
    tables: [&'f Table; 2],
}
impl<'f> crate::application::file_dag::feed::Feed<'f> for Held<'f> {
    fn kinds(
        &self,
        which: crate::domain::operating::types::InputPlace,
    ) -> Result<&[CellKind], OperatingError> {
        Ok(&self.tables[which.0].kinds)
    }
    fn streams(
        &self,
        _which: crate::domain::operating::types::InputPlace,
    ) -> crate::domain::operating::types::Supply {
        crate::domain::operating::types::Supply::AtHand
    }
    fn found(
        &mut self,
        _which: crate::domain::operating::types::InputPlace,
        _rows: crate::domain::table_model::types::RowCount,
    ) {
    }
    fn stream(
        &mut self,
        which: crate::domain::operating::types::InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError> {
        crate::domain::replay::service::scan(
            &Rows::Computed(self.tables[which.0]),
            &mut Vec::new(),
            context,
            emit,
        )
    }
    fn hold<'t>(
        &mut self,
        which: crate::domain::operating::types::InputPlace,
        _context: &mut OperatorContext<'_>,
        _into: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        Ok(Rows::Computed(self.tables[which.0]))
    }
}
fn streamed_and_computed_visits_agree() -> Result<usize, OperatingError> {
    let roots = numbers(1, &[&[1], &[1]]);
    let edges = numbers(4, &[&[1, 2, 1, 12], &[1, 3, 0, 13], &[3, 1, 0, 31]]);
    let op = operation(&[], &[]);
    let computed = compute(&op, &roots, &edges, &mut OperatorScratch::default())?;
    let mut emitted = Table {
        kinds: computed.kinds.clone(),
        ..Table::default()
    };
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    OrderedDfsItem.stream(
        &op,
        &mut Held {
            tables: [&roots, &edges],
        },
        &mut context,
        &mut crate::domain::operating::types::Room::default(),
        &mut crate::domain::replay::types::Collect(&mut emitted),
    )?;
    assert_eq!(computed, emitted);
    Ok(emitted.count.0)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("file_keys_keep_reused_node_ids_separate"),
        word: Word::Succeeds,
        input: || file_keys_keep_reused_nodes_separate,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("replay_preserves_both_inputs_and_visit_rows"),
        word: Word::Succeeds,
        input: || streamed_and_computed_visits_agree,
        expect: Expect::Value(3),
    },
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
        name: CaseName("scratch_reuse_empty_and_old_operator"),
        word: Word::Succeeds,
        input: || scratch_reuse_across_graphs_empty_and_old_operator,
        expect: Expect::Value(1),
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
        name: CaseName("retained_text_uses_exact_text_and_preserves_outputs"),
        word: Word::Succeeds,
        input: || retained_text_keys_and_vertices_preserve_output_cells,
        expect: Expect::Value(6),
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
    Case {
        name: CaseName("nonnumeric_order_cell_refused"),
        word: Word::Refuses,
        input: || nonnumeric_order_cell_is_rejected,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::ShapeMismatch))),
    },
    Case {
        name: CaseName("ordinal_overflow_is_stable"),
        word: Word::Refuses,
        input: || ordinal_overflow_is_checked_after_the_last_representable_visit,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::DfsOrdinalOverflow))
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::OrderedDfs,
    );
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
