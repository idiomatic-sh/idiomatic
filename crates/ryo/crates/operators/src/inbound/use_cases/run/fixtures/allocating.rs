//! The allocation rows, measured: the blocks a file's keyed selects allocate once the worker's buffers are warm, which the
//! allocation test compares for a thousand rows and a hundred thousand; the blocks a seeded closure allocates, compared for
//! a hundred chains and ten thousand; and the blocks the graph kernels allocate over a file's edges, cold and warm.

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
    QueryLiteral, ReachLookup, Side, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The workloads the allocation rows are proved on, within their bounds or past them.
#[derive(Clone, Copy, Debug)]
pub struct Workloads {
    /// Whether each workload is the one past its bound: selects whose answer grows with the rows, the join over a whole
    /// closure, and the graph kernels cold.
    pub past_bound: bool,
}

/// Whether the rows hold for `workloads`. Within the bounds, every row holds: the keyed selects and the seeded closure
/// allocate the same blocks for few rows and many, and the warm graph kernels none. Past them, it holds when any workload
/// past its bound holds its row, so the allocation test's second assertion shows every row able to fail.
#[must_use]
pub fn held(workloads: &Workloads) -> bool {
    let past = workloads.past_bound;
    let selects = selects_held(&KeyedSelects { answering: past });
    let closure = seeded_held(&SeededClosure { fused: !past });
    let kernels =
        graph_kernel_blocks().is_some_and(|[cold, warm]| if past { cold == 0 } else { warm == 0 });
    if past {
        selects || closure || kernels
    } else {
        selects && closure && kernels
    }
}

/// Two selects of `Defines` keyed on its name, so the planner shares one index of the name between them.
#[derive(Clone, Copy, Debug)]
struct KeyedSelects {
    /// Whether the key is a name every fourth row has, so the selects' answer grows with the rows; else no row has it, and
    /// only the index and the lookups allocate.
    answering: bool,
}

/// Whether the row holds for `selects`: a file of a hundred thousand rows allocates the same blocks as a file of a thousand,
/// each measured with buffers warm from a file of the same rows. Selects that do not run do not hold it.
fn selects_held(selects: &KeyedSelects) -> bool {
    match (blocks(selects, 1_000), blocks(selects, 100_000)) {
        (Some(few), Some(many)) => few == many,
        (None, _) | (_, None) => false,
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

/// The blocks one file of `rows` rows allocates in the two keyed selects, its buffers warm from a file of the same rows, or
/// nothing when the selects do not run. The caller's global allocator is `dhat`'s.
fn blocks(selects: &KeyedSelects, rows: usize) -> Option<u64> {
    let _profiler = dhat::Profiler::builder().testing().build();
    let mut resources = TextResources::new(&["kind", "n0", "n1", "n2", "n3", "absent"]);
    let table = defines(rows, &resources);
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let steps = keyed_selects(if selects.answering { "n1" } else { "absent" });
    let emits = [PlanNodeId(0), PlanNodeId(1)];
    let mut buffers = FileBuffers::default();
    emitted(&steps, &emits, &inputs, &mut resources, &mut buffers).ok()?;
    // The warm file ends: the engine releases every node it kept, and the next file starts with their capacity.
    for node in emits {
        buffers.release(node);
    }
    let before = dhat::HeapStats::get().total_blocks;
    let measured = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers);
    let after = dhat::HeapStats::get().total_blocks;
    measured.ok().map(|_| after - before)
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
fn seeded_held(closure: &SeededClosure) -> bool {
    match (
        closure_blocks(closure, 100),
        closure_blocks(closure, 10_000),
    ) {
        (Some(few), Some(many)) => few == many,
        (None, _) | (_, None) => false,
    }
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
/// or nothing when the program does not run. The caller's global allocator is `dhat`'s.
fn closure_blocks(closure: &SeededClosure, count: u32) -> Option<u64> {
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
    emitted(&steps, &emit, &inputs, &mut resources, &mut buffers).ok()?;
    buffers.release(emit[0]);
    let before = dhat::HeapStats::get().total_blocks;
    let measured = emitted(&steps, &emit, &inputs, &mut resources, &mut buffers);
    let after = dhat::HeapStats::get().total_blocks;
    measured.ok().map(|_| after - before)
}

/// The blocks the graph kernels allocate over a file's edges, cold and then warm from a file of the same edges: `reach`
/// from every source, `ordered_walk` from every parent and `components`, each writing into a table whose capacity is kept,
/// as a worker's buffers keep theirs. The caller's global allocator is `dhat`'s.
fn graph_kernel_blocks() -> Option<[u64; 2]> {
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
    let cold = graph_kernels_once(&edges, &inputs, &mut resources, &mut buffers, &mut output)?;
    let warm = graph_kernels_once(&edges, &inputs, &mut resources, &mut buffers, &mut output)?;
    Some([cold, warm])
}

/// The blocks one run of the three graph kernels over `edges` allocates.
fn graph_kernels_once(
    edges: &Table,
    inputs: &TableInputs,
    resources: &mut TextResources,
    buffers: &mut FileBuffers,
    output: &mut Table,
) -> Option<u64> {
    use crate::domain::physical_plan::collections::kernels::components::item::cyclic;
    use crate::domain::physical_plan::collections::kernels::ordered_walk::item::visited;
    use crate::domain::physical_plan::collections::kernels::probe::item::ProbeScratch;
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
        seen,
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
                        row: &mut *row,
                        written: &mut *written,
                        seen: &mut *seen,
                        decisions: &mut decisions,
                    },
                },
                output,
            ),
            1 => visited(&dfs, rows, rows, &mut *walk, output),
            _ => cyclic(&cycles, rows, &mut *index, &mut *cycle_scratch, output),
        };
        if !matches!(fault, Ok(None)) {
            return None;
        }
    }
    let after = dhat::HeapStats::get().total_blocks;
    Some(after - before)
}
