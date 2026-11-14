//! The operators crate's kernels workload.
//!
//! One file's rows of `defines`, planned and run through seam C at a few row counts: selects keyed on one name column (one
//! shared index and its lookups), a projection, a union, an antijoin, an extension, a join and a counted grouping over them.
//! And the graph kernels over `semantic-child` edges: a closure over deep chains fused into its join (1, 10 and 100 seeds
//! over 100,000 edges) and from every source, an ordered depth-first visit and the cycles of 10,000 edges, and a group
//! digest of 100,000 rows. Recorded, not gated.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot run its own program has nothing to measure: the panic is the failure"
)]

use ryo_operators::domain::physical_plan::fixtures::Step;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_operators::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    FileId, Integer, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, ReachLookup, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the workload reads: refinement (column 2) and name (column 3) are texts.
const DEFINES: Relation = Relation::Defines;

/// How many distinct names the rows hold, and how many selects key on one of them.
const NAMES: usize = 43;

/// The rows of `DEFINES`: row `at` names `names[at % NAMES]`, every third one refined as `other`.
fn defines(rows: usize, names: &[String], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(rows * kinds(DEFINES).len());
    for at in 0..rows {
        let tree = Value::Node(node_id(
            FileId(0),
            Position(u32::try_from(at).unwrap_or(u32::MAX)),
        ));
        let name = names.get(at % NAMES).map_or("kind", String::as_str);
        cells.extend([
            tree,
            texts.symbol("kind"),
            texts.symbol(if at % 3 == 0 { "other" } else { "kind" }),
            texts.symbol(name),
            tree,
            Value::File(FileId(0)),
            Value::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
        ]);
    }
    Table {
        kinds: kinds(DEFINES).to_vec(),
        cells,
        count: RowCount(rows),
    }
}

/// A comparison of column `column` with the written text `text`.
fn equal(column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The step of `operation` over `inputs`.
fn step(operation: Operation, inputs: Vec<PlanInput>) -> Step {
    Step { operation, inputs }
}

/// The input naming node `at`, as the operation and the program name it.
fn node(at: usize) -> (Input, PlanInput) {
    let at = u32::try_from(at).unwrap_or(u32::MAX);
    (Input::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)))
}

/// The workload's program: a select keyed on each name, then the other kinds over the first two of them.
fn program(names: &[String]) -> Vec<Step> {
    let relation = (Input::Relation(DEFINES), PlanInput::Relation(DEFINES));
    let mut steps: Vec<Step> = names
        .iter()
        .map(|name| {
            step(
                Operation::Select {
                    from: relation.0,
                    comparisons: vec![equal(3, name)],
                    kept: Vec::new(),
                },
                vec![relation.1],
            )
        })
        .collect();
    let first = steps.len();
    let projected = |at: usize| {
        step(
            Operation::Project {
                from: node(at).0,
                columns: vec![ColumnIndex(3)],
                distinct: RepeatedRows::Dropped,
            },
            vec![node(at).1],
        )
    };
    steps.extend([
        projected(0),
        projected(1),
        step(
            Operation::Union {
                left: node(first).0,
                right: node(first + 1).0,
            },
            vec![node(first).1, node(first + 1).1],
        ),
        step(
            Operation::Antijoin {
                left: relation.0,
                right: node(first + 2).0,
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(0)],
            },
            vec![relation.1, node(first + 2).1],
        ),
        step(
            Operation::Extend {
                from: node(0).0,
                values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
            },
            vec![node(0).1],
        ),
        step(
            Operation::Join {
                left: node(first + 2).0,
                right: relation.0,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(3)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(0),
                    },
                ],
                filter: vec![equal(2, "other")],
                view: Vec::new(),
                reach: None,
            },
            vec![node(first + 2).1, relation.1],
        ),
        step(
            Operation::GroupCount {
                from: relation.0,
                keys: vec![ColumnIndex(3)],
                distinct: None,
            },
            vec![relation.1],
        ),
    ]);
    steps
}

/// The workload over `rows` rows: the program planned and run, every node emitted, its buffers released after.
#[divan::bench(args = [1_000, 100_000])]
fn planned_and_run(bencher: divan::Bencher<'_, '_>, rows: usize) {
    let names: Vec<String> = (0..NAMES).map(|at| format!("name{at}")).collect();
    let texts: Vec<&str> = ["kind", "other"]
        .into_iter()
        .chain(names.iter().map(String::as_str))
        .collect();
    let mut resources = TextResources::new(&texts);
    let table = defines(rows, &names, &resources);
    let inputs = TableInputs::new(&[(DEFINES, &table)]);
    let steps = program(&names);
    let emits: Vec<PlanNodeId> = (0..steps.len())
        .map(|at| PlanNodeId(u32::try_from(at).unwrap_or(u32::MAX)))
        .collect();
    let mut buffers = FileBuffers::default();
    bencher.bench_local(|| {
        let answered = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers)
            .expect("the workload's program runs");
        for emit in &emits {
            buffers.release(*emit);
        }
        answered
    });
}

/// The relation the graph workloads read edges from: parent, child, ordinal (a number), field, file.
const EDGES: Relation = Relation::SemanticChild;

/// Rows of `EDGES` from `pairs` (parent, child, order), the field `kind`, file 0.
fn edge_rows(pairs: impl Iterator<Item = (u32, u32, i64)>, texts: &TextResources) -> Table {
    let mut table = Table { kinds: kinds(EDGES).to_vec(), ..Table::default() };
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
fn run_graph(bencher: divan::Bencher<'_, '_>, steps: &[Step], tables: &[(Relation, &Table)], texts: &mut TextResources) {
    let inputs = TableInputs::new(tables);
    let emit = [PlanNodeId(u32::try_from(steps.len() - 1).unwrap_or(u32::MAX))];
    let mut buffers = FileBuffers::default();
    bencher.bench_local(|| {
        let answered = emitted(steps, &emit, &inputs, texts, &mut buffers).expect("the workload's program runs");
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
                JoinColumn { side: Side::Left, column: ColumnIndex(0) },
                JoinColumn { side: Side::Right, column: ColumnIndex(1) },
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
    let left = edge_rows((0..seeds).map(|chain| (chain * 1_001, 0, 0)), &texts);
    run_graph(bencher, &[seeded_closure()], &[(EDGES, &edges), (DEFINES, &left)], &mut texts);
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
                [(ring * 10 + at, ring * 10 + (at + 1) % 10, order), (ring * 10 + at, ring * 10 + (at + 3) % 10, order + 1)]
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
        operation: Operation::Cycles { edges: Input::Relation(EDGES), source: ColumnIndex(0), destination: ColumnIndex(1) },
        inputs: vec![PlanInput::Relation(EDGES)],
    };
    run_graph(bencher, &[cycles], &[(EDGES, &edges)], &mut texts);
}

/// The digest of each parent's ordinals over 100,000 edges, 1,000 parents of 100 children each.
#[divan::bench]
fn group_digest(bencher: divan::Bencher<'_, '_>) {
    let mut texts = TextResources::new(&["kind"]);
    let edges = edge_rows((0..100_000_u32).map(|at| (at / 100, 100_000 + at, i64::from(at % 7))), &texts);
    let digest = Step {
        operation: Operation::GroupDigest { from: Input::Relation(EDGES), keys: vec![ColumnIndex(0)], over: ColumnIndex(2) },
        inputs: vec![PlanInput::Relation(EDGES)],
    };
    run_graph(bencher, &[digest], &[(EDGES, &edges)], &mut texts);
}
