//! The allocation rows of the kernels, measured: the blocks a file's nodes allocate once the worker's buffers are warm, which
//! the allocation test compares for a thousand rows and a hundred thousand. Each row is a program over `Defines`: two
//! selects sharing one index of its name, and the nodes that drop repeated rows through `hash_index`'s keyed rows.

use super::{TableInputs, TextResources, emitted};
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
pub struct Kernels {
    /// Whether each program's answer grows with the rows: the selects key on a name every fourth row has, and every row is
    /// its own node, so no row repeats; else no row has the key, the rows repeat four rows, and only the index, the lookups
    /// and the keyed rows of what is kept allocate.
    pub answering: bool,
}

/// One allocation row of the kernels.
#[derive(Clone, Copy, Debug)]
enum KernelRow {
    /// Two selects keyed on one column, sharing one index of it.
    KeyedSelects,
    /// A union, an intersection (a probe dropping repeated answers) and a projection dropping repeats.
    RepeatsDropped,
}

/// Whether every row holds for `kernels`: a file of a hundred thousand rows allocates the same blocks as a file of a
/// thousand, each measured with buffers warm from a file of the same rows. Programs that do not run do not hold it.
#[must_use]
pub fn held(kernels: &Kernels) -> bool {
    [KernelRow::KeyedSelects, KernelRow::RepeatsDropped]
        .into_iter()
        .all(|row| match (blocks(row, kernels, 1_000), blocks(row, kernels, 100_000)) {
            (Some(few), Some(many)) => few == many,
            (None, _) | (_, None) => false,
        })
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

/// The blocks one file of `rows` rows allocates in `row`'s program, its buffers warm from a file of the same rows, or nothing
/// when the program does not run. The caller's global allocator is `dhat`'s.
fn blocks(row: KernelRow, kernels: &Kernels, rows: usize) -> Option<u64> {
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
    };
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let emits: Vec<PlanNodeId> = (0..u32::try_from(steps.len()).unwrap_or(u32::MAX))
        .map(PlanNodeId)
        .collect();
    let mut buffers = FileBuffers::default();
    let _warm = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers).ok()?;
    // The warm file ends: the engine releases every node it kept, and the next file starts with their capacity.
    for node in &emits {
        buffers.release(*node);
    }
    let before = dhat::HeapStats::get().total_blocks;
    let measured = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers);
    let after = dhat::HeapStats::get().total_blocks;
    measured.ok().map(|_| after - before)
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
    let defines = (Input::Relation(Relation::Defines), PlanInput::Relation(Relation::Defines));
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
