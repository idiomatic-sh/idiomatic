//! The kernels' allocation rows: two selects sharing one index of a name, the nodes that drop repeated rows through
//! `hash_index`'s keyed rows, and the span kinds of `text_map`, each measured for a thousand rows and a hundred thousand.
use super::super::{TableInputs, TextResources, emitted};
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Offset, Position, Relation, Span, Value};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, LiteralText, Operand, PlanComparison, QueryLiteral,
    RepeatedRows, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The programs of the kernels' allocation rows, each answering little or answering more for more rows.
#[derive(Clone, Copy, Debug)]
pub(super) struct Kernels {
    /// Whether each program's answer grows with the rows: the selects key on a name every fourth row has, every row is its
    /// own node, so no row repeats, and words are minted from every row; else no row has the key, the rows repeat four rows,
    /// the span kinds write into cells reserved once, and only the index, the lookups and the keyed rows of what is kept
    /// allocate.
    pub(super) answering: bool,
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
pub(super) fn kernels_held(kernels: &Kernels) -> bool {
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
    #[expect(
        clippy::expect_used,
        reason = "a program that faults proves no allocation row: the panic is the failure"
    )]
    let _warm =
        emitted(&steps, &emits, &inputs, &mut resources, &mut buffers).expect("the warm file runs");
    // The warm file ends: the engine releases every node it kept, and the next file starts with their capacity.
    for node in &emits {
        buffers.release(*node);
    }
    let before = dhat::HeapStats::get().total_blocks;
    let measured = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers);
    let after = dhat::HeapStats::get().total_blocks;
    #[expect(
        clippy::expect_used,
        reason = "a program that faults proves no allocation row: the panic is the failure"
    )]
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
