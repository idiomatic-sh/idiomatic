//! The allocation row of the shared hash index, measured: the blocks a file's keyed selects allocate once the worker's buffers
//! are warm, which the allocation test compares for a thousand rows and a hundred thousand.

use super::{Step, TableInputs, TextResources, emitted};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Offset, Position, Relation, Span, Value};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// Two selects of `Defines` keyed on its name, so the planner shares one index of the name between them.
#[derive(Clone, Copy, Debug)]
pub struct KeyedSelects {
    /// Whether the key is a name every fourth row has, so the selects' answer grows with the rows; else no row has it, and
    /// only the index and the lookups allocate.
    pub answering: bool,
}

/// Whether the row holds for `selects`: a file of a hundred thousand rows allocates the same blocks as a file of a thousand,
/// each measured with buffers warm from a file of the same rows. Selects that do not run do not hold it.
#[must_use]
pub fn held(selects: &KeyedSelects) -> bool {
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
        let node = Value::Node(node_id(FileId(0), Position(u32::try_from(at).unwrap_or(u32::MAX))));
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
    let before = dhat::HeapStats::get().total_blocks;
    let measured = emitted(&steps, &emits, &inputs, &mut resources, &mut buffers);
    let after = dhat::HeapStats::get().total_blocks;
    measured.ok().map(|_| after - before)
}
