//! The allocation rows, measured: the blocks the kernels' programs over `Defines` allocate for a thousand rows and a hundred
//! thousand (two selects sharing one index of its name, the nodes that drop repeated rows through `hash_index`'s keyed rows,
//! and the span kinds of `text_map`, which write one row per row into cells reserved once), a seeded closure for a hundred
//! chains and ten thousand, and the graph kernels over a file's edges, cold and warm.

use super::{TableInputs, TextResources, emitted};
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    FileId, Integer, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, ReachLookup, RepeatedRows, Side, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The workloads the allocation rows are proved on, within their bounds or past them.
#[derive(Clone, Copy, Debug)]
pub struct Workloads {
    /// Whether each workload is the one past its bound: kernel programs whose answer grows with the rows, the join over a
    /// whole closure, and the graph kernels cold.
    pub past_bound: bool,
}

/// Whether the rows hold for `workloads`. Within the bounds, every row holds: the keyed selects and the seeded closure
/// allocate the same blocks for few rows and many, and the warm graph kernels none. Past them, it holds when any workload
/// past its bound holds its row, so the allocation test's second assertion shows every row able to fail. Every workload
/// runs to the end before any is judged.
///
/// # Panics
///
/// When a workload faults: a program that does not run proves no row, and is no budget breach.
#[must_use]
pub fn held(workloads: &Workloads) -> bool {
    let past = workloads.past_bound;
    let kernels = kernels_held(&Kernels { answering: past });
    let closure = seeded_held(&SeededClosure { fused: !past });
    let graph = {
        let [cold, warm] = graph_kernel_blocks();
        if past { cold == 0 } else { warm == 0 }
    };
    if past {
        kernels || closure || graph
    } else {
        kernels && closure && graph
    }
}

/// The programs of the kernels' allocation rows, each answering little or answering more for more rows.
#[derive(Clone, Copy, Debug)]
struct Kernels {
    /// Whether each program's answer grows with the rows: the selects key on a name every fourth row has, every row is its
    /// own node, so no row repeats, and words are minted from every row; else no row has the key, the rows repeat four rows,
    /// the span kinds write into cells reserved once, and only the index, the lookups and the keyed rows of what is kept
    /// allocate.
    answering: bool,
}

/// One allocation row of the kernels.
#[derive(Clone, Copy, Debug)]
enum KernelRow {
    /// Two selects keyed on one column, sharing one index of it.
    KeyedSelects,
    /// A union, an intersection (a probe dropping repeated answers) and a projection dropping repeats.
    RepeatsDropped,
    /// The span kinds of `text_map`, one row per row: span-bounds and span-between of each row's span.
    SpanKinds,
}

/// Whether `kernels` holds the kernels' rows, each judged on its own: a row holds when a file of a hundred thousand rows
/// allocates the same blocks as a file of a thousand, each measured with buffers warm from a file of the same rows. The
/// bounded programs hold only when every row holds; the answering programs hold when any row still holds.
///
/// # Panics
///
/// When a program faults: a program that does not run proves no row.
fn kernels_held(kernels: &Kernels) -> bool {
    let rows = [
        KernelRow::KeyedSelects,
        KernelRow::RepeatsDropped,
        KernelRow::SpanKinds,
    ];
    let holds = |row: &KernelRow| blocks(*row, kernels, 1_000) == blocks(*row, kernels, 100_000);
    if kernels.answering {
        rows.iter().any(holds)
    } else {
        rows.iter().all(holds)
    }
}

/// The two keyed selects, keyed on `name`.
fn keyed_selects(name: &str) -> Vec<Step> {
    (0..2)
        .map(|_| Step {
            operation: Operation::Select {
                from: Input::Relation(Relation::Defines),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(3)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            },
            inputs: vec![PlanInput::Relation(Relation::Defines)],
        })
        .collect()
}

/// `rows` rows of `Defines`, each named by one of four names.
fn defines(rows: usize, resources: &TextResources) -> Table {
    let names = ["n0", "n1", "n2", "n3"];
    let mut cells = Vec::with_capacity(rows * kinds(Relation::Defines).len());
    for (at, name) in (0..rows).zip(names.iter().cycle()) {
        let node = Value::Node(node_id(
            FileId(0),
            Position(u32::try_from(at).unwrap_or(u32::MAX)),
        ));
        cells.extend([
            node,
            resources.symbol("kind"),
            resources.symbol("kind"),
            resources.symbol(name),
            node,
            Value::File(FileId(0)),
            Value::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
        ]);
    }
    Table {
        kinds: kinds(Relation::Defines).to_vec(),
        cells,
        count: RowCount(rows),
    }
}

/// The blocks one file of `rows` rows allocates in `row`'s program, its buffers warm from a file of the same rows. The
/// caller's global allocator is `dhat`'s.
///
/// # Panics
///
/// When the program faults.
fn blocks(row: KernelRow, kernels: &Kernels, rows: usize) -> u64 {
    let _profiler = dhat::Profiler::builder().testing().build();
    let mut resources = TextResources::new(&["kind", "n0", "n1", "n2", "n3", "absent"]);
    let mut table = defines(rows, &resources);
    let steps = match row {
        KernelRow::KeyedSelects => keyed_selects(if kernels.answering { "n1" } else { "absent" }),
        KernelRow::RepeatsDropped => {
            if !kernels.answering {
                repeated(&mut table);
            }
            repeats_dropped()
        }
        KernelRow::SpanKinds => span_kinds(kernels.answering),
    };
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let emits: Vec<PlanNodeId> = (0..u32::try_from(steps.len()).unwrap_or(u32::MAX))
        .map(PlanNodeId)
        .collect();
    let mut buffers = FileBuffers::default();
    #[expect(clippy::expect_used, reason = "a program that faults proves no allocation row: the panic is the failure")]
    let _warm = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers).expect("the warm file runs");
    // The warm file ends: the engine releases every node it kept, and the next file starts with their capacity.
    for node in &emits {
        buffers.release(*node);
    }
    let before = dhat::HeapStats::get().total_blocks;
    let measured = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers);
    let after = dhat::HeapStats::get().total_blocks;
    #[expect(clippy::expect_used, reason = "a program that faults proves no allocation row: the panic is the failure")]
    let _measured = measured.expect("the measured file runs");
    after - before
}

/// Makes `table`'s rows four rows repeated: each row's node becomes the node of its name.
fn repeated(table: &mut Table) {
    let width = table.kinds.len();
    for (row, at) in table.cells.chunks_mut(width).zip((0..4).cycle()) {
        let node = Value::Node(node_id(FileId(0), Position(at)));
        for column in [0, 4] {
            if let Some(cell) = row.get_mut(column) {
                *cell = node;
            }
        }
    }
}

/// A union of `Defines` with itself, an intersection of it with itself, and its node and name with repeats dropped.
fn repeats_dropped() -> Vec<Step> {
    let defines = (
        Input::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    );
    let both = |operation: Operation| Step {
        operation,
        inputs: vec![defines.1, defines.1],
    };
    vec![
        both(Operation::Union {
            left: defines.0,
            right: defines.0,
        }),
        both(Operation::Intersection {
            left: defines.0,
            right: defines.0,
        }),
        Step {
            operation: Operation::Project {
                from: defines.0,
                columns: vec![ColumnIndex(0), ColumnIndex(3)],
                distinct: RepeatedRows::Dropped,
            },
            inputs: vec![defines.1],
        },
    ]
}

/// Span-bounds and span-between of each row's span; or, answering, the words of each row's name, minted one by one.
fn span_kinds(answering: bool) -> Vec<Step> {
    let defines = (
        Input::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    );
    let over = |operation: Operation| Step {
        operation,
        inputs: vec![defines.1],
    };
    if answering {
        return vec![over(Operation::Words {
            from: defines.0,
            text: ColumnIndex(3),
        })];
    }
    vec![
        over(Operation::SpanBounds {
            from: defines.0,
            span: ColumnIndex(6),
        }),
        over(Operation::SpanBetween {
            from: defines.0,
            start: ColumnIndex(6),
            end: ColumnIndex(6),
        }),
    ]
}

/// The relation the graph cases read edges from: parent, child, ordinal (a number), field, file.
const EDGES: Relation = Relation::SemanticChild;

/// A closure joined with left rows binding the first node of two chains of ten edges: fused into its join, as the query
/// planner fuses it (`Join { reach }`), or, the counter-example, the join over the whole closure.
#[derive(Clone, Copy, Debug)]
struct SeededClosure {
    /// Whether the closure is fused into its join; else the join reads the whole closure.
    fused: bool,
}

/// Whether a file of ten thousand chains allocates the same blocks as a file of a hundred, for the same two bound chains,
/// each measured with buffers warm from a file of the same rows. A whole closure grows with the graph and does not hold it.
///
/// # Panics
///
/// When the program faults.
fn seeded_held(closure: &SeededClosure) -> bool {
    closure_blocks(closure, 100) == closure_blocks(closure, 10_000)
}

/// `pairs` rows of `EDGES`, parent to child, each of ordinal 0, the field `f`, file 0.
fn edge_rows(pairs: impl Iterator<Item = (u32, u32)>, resources: &TextResources) -> Table {
    let mut table = Table {
        kinds: kinds(EDGES).to_vec(),
        ..Table::default()
    };
    for (parent, child) in pairs {
        table.cells.extend([
            Value::Node(node_id(FileId(0), Position(parent))),
            Value::Node(node_id(FileId(0), Position(child))),
            Value::Number(Integer(0)),
            resources.symbol("f"),
            Value::File(FileId(0)),
        ]);
        table.count = RowCount(table.count.0 + 1);
    }
    table
}

/// `chains` chains of ten edges, chain `c` from node `11c` to `11c + 10`.
fn chains(chains: u32, resources: &TextResources) -> Table {
    edge_rows(
        (0..chains).flat_map(|chain| (chain * 11..chain * 11 + 10).map(|at| (at, at + 1))),
        resources,
    )
}

/// The closure's program: the join of the left rows (read from `Defines`) with the closure of the edges on its source,
/// answering the left node and the reached one; fused, or over the whole closure node.
fn closure_steps(fused: bool) -> Vec<Step> {
    let join = |right: Input, reach| Operation::Join {
        left: Input::Relation(Relation::Defines),
        right,
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
        reach,
    };
    if fused {
        let reach = ReachLookup {
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: Vec::new(),
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
        };
        return vec![Step {
            operation: join(Input::Relation(EDGES), Some(reach)),
            inputs: vec![
                PlanInput::Relation(Relation::Defines),
                PlanInput::Relation(EDGES),
            ],
        }];
    }
    vec![
        Step {
            operation: Operation::Closure {
                edges: Input::Relation(EDGES),
                keys: Vec::new(),
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            inputs: vec![PlanInput::Relation(EDGES)],
        },
        Step {
            operation: join(
                Input::Step(ryo_values::domain::plan_parts::types::StepId(0)),
                None,
            ),
            inputs: vec![
                PlanInput::Relation(Relation::Defines),
                PlanInput::Node(PlanNodeId(0)),
            ],
        },
    ]
}

/// The blocks one file of `count` chains allocates in the closure's program, its buffers warm from a file of the same rows,
/// under the caller's `dhat` allocator.
fn closure_blocks(closure: &SeededClosure, count: u32) -> u64 {
    let _profiler = dhat::Profiler::builder().testing().build();
    let mut resources = TextResources::new(&["f"]);
    let edges = chains(count, &resources);
    // The left rows bind the first node of chains 3 and 70.
    let left = edge_rows([(33, 0), (770, 0)].into_iter(), &resources);
    let inputs = TableInputs::new(&[(EDGES, &edges), (Relation::Defines, &left)]);
    let steps = closure_steps(closure.fused);
    let emit = [PlanNodeId(
        u32::try_from(steps.len() - 1).unwrap_or(u32::MAX),
    )];
    let mut buffers = FileBuffers::default();
    #[expect(
        clippy::expect_used,
        reason = "a program that faults proves no allocation row: the panic is the failure"
    )]
    let _warm =
        emitted(&steps, &emit, &inputs, &mut resources, &mut buffers).expect("the warm file runs");
    buffers.release(emit[0]);
    let before = dhat::HeapStats::get().total_blocks;
    let measured = emitted(&steps, &emit, &inputs, &mut resources, &mut buffers);
    let after = dhat::HeapStats::get().total_blocks;
    #[expect(
        clippy::expect_used,
        reason = "a program that faults proves no allocation row: the panic is the failure"
    )]
    let _measured = measured.expect("the measured file runs");
    after - before
}

/// The blocks the graph kernels allocate over a file's edges, cold and then warm from a file of the same edges: `reach`
/// from every source, `ordered_walk` from every parent and `components`, each writing into a table whose capacity is kept,
/// as a worker's buffers keep theirs. The caller's global allocator is `dhat`'s.
fn graph_kernel_blocks() -> [u64; 2] {
    let _profiler = dhat::Profiler::builder().testing().build();
    let mut resources = TextResources::new(&["f"]);
    // Rings of ten nodes, each with a chord: cycles for `components`, and walks that meet entities again.
    let mut edges = edge_rows(
        (0..100_u32).flat_map(|ring| {
            (0..10).flat_map(move |at| {
                [
                    (ring * 10 + at, ring * 10 + (at + 1) % 10),
                    (ring * 10 + at, ring * 10 + (at + 3) % 10),
                ]
            })
        }),
        &resources,
    );
    // Each edge its own order among its source's edges.
    for (at, cell) in edges.cells.chunks_mut(5).enumerate() {
        if let Some(order) = cell.get_mut(2) {
            *order = Value::Number(Integer(i64::try_from(at).unwrap_or(i64::MAX)));
        }
    }
    let inputs = TableInputs::new(&[(EDGES, &edges)]);
    let mut buffers = FileBuffers::default();
    let mut output = Table::default();
    let cold = graph_kernels_once(&edges, &inputs, &mut resources, &mut buffers, &mut output);
    let warm = graph_kernels_once(&edges, &inputs, &mut resources, &mut buffers, &mut output);
    [cold, warm]
}

/// The blocks one run of the three graph kernels over `edges` allocates.
fn graph_kernels_once(
    edges: &Table,
    inputs: &TableInputs,
    resources: &mut TextResources,
    buffers: &mut FileBuffers,
    output: &mut Table,
) -> u64 {
    use crate::domain::physical_plan::collections::kernels::components::item::cyclic;
    use crate::domain::physical_plan::collections::kernels::ordered_walk::item::visited;
    use crate::domain::physical_plan::collections::kernels::probe::item::{IndexBuild, ProbeScratch};
    use crate::domain::physical_plan::collections::kernels::reach::item::{
        ReachScratch, Starts, reached,
    };
    use crate::domain::physical_plan::types::{InputRows, KernelRun};
    let rows = InputRows::Table(edges);
    let graph = crate::domain::closure::service::lower(&[], ColumnIndex(0), ColumnIndex(1));
    let dfs =
        crate::domain::ordered_dfs::service::lower(crate::domain::ordered_dfs::types::DfsShape {
            root: ColumnIndex(0),
            edge_columns: [
                ColumnIndex(0),
                ColumnIndex(1),
                ColumnIndex(2),
                ColumnIndex(2),
            ],
            root_keys: &[],
            edge_keys: &[],
        });
    let cycles = crate::domain::cycles::service::lower(ColumnIndex(0), ColumnIndex(1));
    let matching = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
    let mut decisions =
        crate::domain::physical_plan::types::file_buffers::value::NodeDecisions::default();
    let before = dhat::HeapStats::get().total_blocks;
    let mut run = KernelRun {
        inputs,
        resources,
        matcher: crate::domain::patterns::types::PatternMatcher(&matching),
    };
    let FileBuffers {
        index,
        second,
        walk,
        cycles: cycle_scratch,
        row,
        written,
        repeats,
        ..
    } = buffers;
    for kernel in 0..3 {
        output.cells.clear();
        output.count = RowCount(0);
        let fault = match kernel {
            0 => reached(
                &graph,
                rows,
                Starts::Every,
                &mut run,
                ReachScratch {
                    index: &mut *index,
                    walk: &mut *walk,
                    probe: ProbeScratch {
                        index: &mut *second,
                        build: IndexBuild::Builds,
                        row: &mut *row,
                        written: &mut *written,
                        repeats: &mut *repeats,
                        decisions: &mut decisions,
                    },
                },
                output,
            ),
            1 => visited(&dfs, rows, rows, &run.resources.texts(), &mut *walk, output),
            _ => cyclic(&cycles, rows, &mut *index, &mut *cycle_scratch, output),
        };
        #[expect(
            clippy::expect_used,
            reason = "a program that faults proves no allocation row: the panic is the failure"
        )]
        let _ran = fault.expect("the graph kernel runs");
    }
    let after = dhat::HeapStats::get().total_blocks;
    after - before
}
