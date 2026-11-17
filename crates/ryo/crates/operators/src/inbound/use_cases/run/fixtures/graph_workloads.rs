//! The graph workloads' allocation rows: a seeded closure for a hundred chains and ten thousand, and the graph kernels over a
//! file's edges, cold and warm.
use super::{TableInputs, TextResources, emitted};
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::shared_index::value::IndexBuild;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, ReachLookup, Side};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the graph cases read edges from: parent, child, ordinal (a number), field, file.
const EDGES: Relation = Relation::SemanticChild;

/// A closure joined with left rows binding the first node of two chains of ten edges: fused into its join, as the query
/// planner fuses it (`Join { reach }`), or, the counter-example, the join over the whole closure.
#[derive(Clone, Copy, Debug)]
pub(super) struct SeededClosure {
    /// Whether the closure is fused into its join; else the join reads the whole closure.
    pub(super) fused: bool,
}

/// Whether a file of ten thousand chains allocates the same blocks as a file of a hundred, for the same two bound chains,
/// each measured with buffers warm from a file of the same rows. A whole closure grows with the graph and does not hold it.
///
/// # Panics
///
/// When the program faults.
pub(super) fn seeded_held(closure: &SeededClosure) -> bool {
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

/// Edge rows as left rows of Defines' shape, each its parent as its node (columns 0 and 4) and its field as its name
/// (columns 1 to 3), lent whole: a relation's rows are never lent short of its kinds.
pub fn defines_of_edges(edges: &Table) -> Table {
    let width = kinds(EDGES).len();
    let span = Value::Span(Span {
        start: Offset(0),
        end: Offset(0),
    });
    Table {
        kinds: kinds(Relation::Defines).to_vec(),
        cells: edges
            .cells
            .chunks(width)
            .flat_map(|row| match row {
                [parent, _, _, field, file] => {
                    vec![*parent, *field, *field, *field, *parent, *file, span]
                }
                _ => Vec::new(),
            })
            .collect(),
        count: edges.count,
    }
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
    let left = defines_of_edges(&edge_rows([(33, 0), (770, 0)].into_iter(), &resources));
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
/// from every source, `ordered_walk` from every parent, `components`, and `queued_walk` from every parent over the one state
/// of the edges' field, each writing into a table whose capacity is kept, as a worker's buffers keep theirs. The caller's
/// global allocator is `dhat`'s.
pub(super) fn graph_kernel_blocks() -> [u64; 2] {
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
    // The worklist's lattice: the field's one state, below itself.
    let field = resources.symbol("f");
    let lattice = Table {
        kinds: vec![CellKind::Symbol, CellKind::Symbol],
        cells: vec![field, field],
        count: RowCount(1),
    };
    let tables = [&edges, &lattice];
    let mut buffers = FileBuffers::default();
    let mut output = Table::default();
    let (cold, cold_rows) =
        graph_kernels_once(tables, &inputs, &mut resources, &mut buffers, &mut output);
    let (warm, warm_rows) =
        graph_kernels_once(tables, &inputs, &mut resources, &mut buffers, &mut output);
    // A kernel that answers no rows allocates nothing and proves nothing: each answers rows over the rings, the same warm
    // as cold.
    assert!(
        cold_rows.iter().all(|rows| rows.0 > 0),
        "every graph kernel (reach, ordered walk, components, worklist) answers rows over the rings: {cold_rows:?}"
    );
    assert_eq!(
        cold_rows, warm_rows,
        "the warm run answers the cold run's rows"
    );
    [cold, warm]
}

/// The blocks one run of the four graph kernels over the edges allocates, the worklist over the lattice, and the rows each
/// answered.
///
/// # Panics
///
/// When a kernel faults, or answers a fault of a row.
fn graph_kernels_once(
    [edges, lattice]: [&Table; 2],
    inputs: &TableInputs,
    resources: &mut TextResources,
    buffers: &mut FileBuffers,
    output: &mut Table,
) -> (u64, [RowCount; 4]) {
    use crate::domain::physical_plan::collections::kernels::components::item::cyclic;
    use crate::domain::physical_plan::collections::kernels::ordered_walk::item::visited;
    use crate::domain::physical_plan::collections::kernels::probe_right::item::ProbeScratch;
    use crate::domain::physical_plan::collections::kernels::queued_walk::item::{
        WorklistInputs, worklisted,
    };
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
    let worklist =
        crate::domain::worklist::service::lower(crate::domain::worklist::types::WorklistShape {
            root: ColumnIndex(0),
            state: ColumnIndex(3),
            edge_columns: [
                ColumnIndex(0),
                ColumnIndex(1),
                ColumnIndex(2),
                ColumnIndex(3),
                ColumnIndex(3),
            ],
            lattice_columns: [ColumnIndex(0), ColumnIndex(1)],
            root_keys: &[],
            edge_keys: &[],
        });
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
        found,
        worklist: worklist_scratch,
        ..
    } = buffers;
    let (mut kept, mut left_on, mut right_on) = (Table::default(), Vec::new(), Vec::new());
    let mut answered = [RowCount(0); 4];
    for kernel in 0..4 {
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
                        kept: &mut kept,
                        whole: [&mut left_on, &mut right_on],
                        row: &mut *row,
                        written: &mut *written,
                        repeats: &mut *repeats,
                        decisions: &mut decisions,
                        found: &mut *found,
                    },
                },
                output,
            ),
            1 => visited(&dfs, rows, rows, &run.resources.texts(), &mut *walk, output),
            2 => cyclic(&cycles, rows, &mut *index, &mut *cycle_scratch, output),
            _ => worklisted(
                &worklist,
                WorklistInputs {
                    roots: rows,
                    edges: rows,
                    lattice: InputRows::Table(lattice),
                },
                &run.resources.texts(),
                &mut *worklist_scratch,
                &mut *repeats,
                output,
            ),
        };
        // A kernel that faults, or answers a row's fault, proves no allocation row: the panic is the failure.
        assert!(
            matches!(fault, Ok(None)),
            "graph kernel {kernel} answered {fault:?}"
        );
        if let Some(rows) = answered.get_mut(kernel) {
            *rows = output.count;
        }
    }
    let after = dhat::HeapStats::get().total_blocks;
    (after - before, answered)
}
