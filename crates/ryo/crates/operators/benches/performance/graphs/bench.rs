//! The operators crate's graph workload.
//!
//! The graph kernels over `semantic-child` edges, planned and run through seam C: a closure over deep chains fused into its
//! join (1, 10 and 100 seeds over 100,000 edges) and from every source, an ordered depth-first visit and the cycles of 10,000
//! edges, and a group digest of 100,000 rows. Recorded, not gated.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot run its own program has nothing to measure: the panic is the failure"
)]

use ryo_operators::domain::physical_plan::fixtures::Step;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_operators::inbound::use_cases::run::fixtures::graph_workloads::defines_of_edges;
use ryo_operators::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, ReachLookup, Side};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the left rows of a seeded closure are read from.
const DEFINES: Relation = Relation::Defines;

/// The relation the graph workloads read edges from: parent, child, ordinal (a number), field, file.
const EDGES: Relation = Relation::SemanticChild;

/// Rows of `EDGES` from `pairs` (parent, child, order), the field `kind`, file 0.
fn edge_rows(pairs: impl Iterator<Item = (u32, u32, i64)>, texts: &TextResources) -> Table {
    let mut table = Table {
        kinds: kinds(EDGES).to_vec(),
        ..Table::default()
    };
    for (parent, child, order) in pairs {
        table.cells.extend([
            Value::Node(node_id(FileId(0), Position(parent))),
            Value::Node(node_id(FileId(0), Position(child))),
            Value::Number(Integer(order)),
            texts.symbol("kind"),
            Value::File(FileId(0)),
        ]);
        table.count = RowCount(table.count.0 + 1);
    }
    table
}

/// `chains` chains of `length` edges, chain `c` from node `c * (length + 1)`.
fn chains(chains: u32, length: u32, texts: &TextResources) -> Table {
    edge_rows(
        (0..chains).flat_map(|chain| {
            let first = chain * (length + 1);
            (first..first + length).map(|at| (at, at + 1, 0))
        }),
        texts,
    )
}

/// Runs `steps` over `tables` again and again, emitting the last node and releasing it after.
fn run_graph(
    bencher: divan::Bencher<'_, '_>,
    steps: &[Step],
    tables: &[(Relation, &Table)],
    texts: &mut TextResources,
) {
    let inputs = TableInputs::new(tables);
    let emit = [PlanNodeId(
        u32::try_from(steps.len() - 1).unwrap_or(u32::MAX),
    )];
    let mut buffers = FileBuffers::default();
    bencher.bench_local(|| {
        let answered = emitted(steps, &emit, &inputs, texts, &mut buffers)
            .expect("the workload's program runs");
        buffers.release(emit[0]);
        answered
    });
}

/// The join of the left rows (read from `Defines`) with the closure of the edges on its source, as the query planner
/// fuses it: the seeded closure.
fn seeded_closure() -> Step {
    Step {
        operation: Operation::Join {
            left: Input::Relation(DEFINES),
            right: Input::Relation(EDGES),
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
            filter: Vec::new(),
            view: Vec::new(),
            reach: Some(ReachLookup {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
                keys: Vec::new(),
                columns: vec![ColumnIndex(0), ColumnIndex(1)],
            }),
        },
        inputs: vec![PlanInput::Relation(DEFINES), PlanInput::Relation(EDGES)],
    }
}

/// A closure over a deep chain fused into its join: 100 chains of 1,000 edges (100,000 edges), the left rows binding the
/// first node of `seeds` of them. Each bound chain is walked once; the others never.
#[divan::bench(args = [1, 10, 100])]
fn seeded_closure_over_deep_chains(bencher: divan::Bencher<'_, '_>, seeds: u32) {
    let mut texts = TextResources::new(&["kind"]);
    let edges = chains(100, 1_000, &texts);
    let left = defines_of_edges(&edge_rows(
        (0..seeds).map(|chain| (chain * 1_001, 0, 0)),
        &texts,
    ));
    run_graph(
        bencher,
        &[seeded_closure()],
        &[(EDGES, &edges), (DEFINES, &left)],
        &mut texts,
    );
}

/// A closure from every source over deep chains: 10 chains of `length` edges, each node reaching the rest of its chain.
#[divan::bench(args = [100, 300])]
fn whole_closure_over_deep_chains(bencher: divan::Bencher<'_, '_>, length: u32) {
    let mut texts = TextResources::new(&["kind"]);
    let edges = chains(10, length, &texts);
    let closure = Step {
        operation: Operation::Closure {
            edges: Input::Relation(EDGES),
            keys: Vec::new(),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        inputs: vec![PlanInput::Relation(EDGES)],
    };
    run_graph(bencher, &[closure], &[(EDGES, &edges)], &mut texts);
}

/// Rings of ten nodes with a chord each, `rings * 20` edges, each edge its own order: cycles to find and walks that meet
/// entities again.
fn chorded_rings(rings: u32, texts: &TextResources) -> Table {
    edge_rows(
        (0..rings).flat_map(|ring| {
            (0..10).flat_map(move |at| {
                let order = i64::from(ring * 20 + at * 2);
                [
                    (ring * 10 + at, ring * 10 + (at + 1) % 10, order),
                    (ring * 10 + at, ring * 10 + (at + 3) % 10, order + 1),
                ]
            })
        }),
        texts,
    )
}

/// An ordered depth-first visit from every parent over 10,000 edges (500 chorded rings).
#[divan::bench]
fn ordered_walk_over_rings(bencher: divan::Bencher<'_, '_>) {
    let mut texts = TextResources::new(&["kind"]);
    let edges = chorded_rings(500, &texts);
    let walk = Step {
        operation: Operation::OrderedDfs {
            from: Input::Relation(EDGES),
            edges: Input::Relation(EDGES),
            root: ColumnIndex(0),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            order: ColumnIndex(2),
            edge: ColumnIndex(2),
            root_keys: Vec::new(),
            edge_keys: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(EDGES), PlanInput::Relation(EDGES)],
    };
    run_graph(bencher, &[walk], &[(EDGES, &edges)], &mut texts);
}

/// The cycles of 10,000 edges (500 chorded rings).
#[divan::bench]
fn components_of_rings(bencher: divan::Bencher<'_, '_>) {
    let mut texts = TextResources::new(&["kind"]);
    let edges = chorded_rings(500, &texts);
    let cycles = Step {
        operation: Operation::Cycles {
            edges: Input::Relation(EDGES),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        inputs: vec![PlanInput::Relation(EDGES)],
    };
    run_graph(bencher, &[cycles], &[(EDGES, &edges)], &mut texts);
}

/// The digest of each parent's ordinals over 100,000 edges, 1,000 parents of 100 children each.
#[divan::bench]
fn group_digest(bencher: divan::Bencher<'_, '_>) {
    let mut texts = TextResources::new(&["kind"]);
    let edges = edge_rows(
        (0..100_000_u32).map(|at| (at / 100, 100_000 + at, i64::from(at % 7))),
        &texts,
    );
    let digest = Step {
        operation: Operation::GroupDigest {
            from: Input::Relation(EDGES),
            keys: vec![ColumnIndex(0)],
            over: ColumnIndex(2),
        },
        inputs: vec![PlanInput::Relation(EDGES)],
    };
    run_graph(bencher, &[digest], &[(EDGES, &edges)], &mut texts);
}
