//! The rows the run use case's relational cases read and show: a `DEFINES` row per (refinement, name) pair of the file's
//! texts, the steps over them, and their answers as text.
use super::cases::{DEFINES, defines, input_of};
use super::{TableInputs, TextResources, emitted};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Value as Cell};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, RepeatedRows, Side};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;

/// The texts of the file the cases read: the refinements and names of their `DEFINES` rows.
pub(crate) const TEXTS: &[&str] = &["kind", "r1", "r2", "a", "b", "zzz"];

/// One cell as the cases show it: a node by its place in file 0, a text by the file's text, a number as itself.
pub(crate) fn cell(value: Cell, texts: &TextResources) -> String {
    if let Some(at) = (0..16).find(|at| value == Cell::Node(node_id(FileId(0), Position(*at)))) {
        return format!("n{at}");
    }
    if let Some(text) = TEXTS.iter().find(|text| texts.symbol(text) == value) {
        return (*text).to_owned();
    }
    if let Cell::Number(Integer(number)) = value {
        number.to_string()
    } else {
        format!("{value:?}")
    }
}

/// The rows of `table`, a row's cells spaced, rows apart by ` | `; a row of no cell is `-`.
pub(crate) fn shown(table: &Table, texts: &TextResources) -> String {
    let width = table.kinds.len();
    (0..table.count.0)
        .map(|row| {
            if width == 0 {
                return "-".to_owned();
            }
            table
                .cells
                .get(row * width..(row + 1) * width)
                .unwrap_or_default()
                .iter()
                .map(|value| cell(*value, texts))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// What `steps` answer at `emits` over `DEFINES` rows of `pairs`, shown node by node.
pub(crate) fn answered(
    steps: &[Step],
    emits: &[u32],
    pairs: &[(&str, &str)],
) -> Result<String, PortableOperatorFault> {
    let mut texts = TextResources::new(TEXTS);
    let rows = defines(pairs, &texts);
    let inputs = TableInputs::new(&[(DEFINES, &rows)]);
    let emits: Vec<PlanNodeId> = emits.iter().copied().map(PlanNodeId).collect();
    let tables = emitted(
        steps,
        &emits,
        &inputs,
        &mut texts,
        &mut FileBuffers::default(),
    )?;
    Ok(tables
        .iter()
        .map(|table| shown(table, &texts))
        .collect::<Vec<_>>()
        .join("; "))
}

/// The input of a step that reads `DEFINES`.
pub(crate) fn relation() -> PlanInput {
    PlanInput::Relation(DEFINES)
}

/// The input of a step that reads node `at`.
pub(crate) fn node(at: u32) -> PlanInput {
    PlanInput::Node(PlanNodeId(at))
}

/// A projection of `from`'s `columns`, dropping repeated rows when `distinct` says.
pub(crate) fn project(from: PlanInput, columns: &[u32], distinct: RepeatedRows) -> Step {
    Step {
        operation: Operation::Project {
            from: input_of(from),
            columns: columns.iter().copied().map(ColumnIndex).collect(),
            distinct,
        },
        inputs: vec![from],
    }
}

/// A step reading `left` then `right`.
pub(crate) fn both(operation: Operation, left: PlanInput, right: PlanInput) -> Step {
    Step {
        operation,
        inputs: vec![left, right],
    }
}

/// The join of `left` with `right` on one key each, answering the columns `output` names.
pub(crate) fn join(
    left: (PlanInput, u32),
    right: (PlanInput, u32),
    output: &[(Side, u32)],
) -> Step {
    both(
        Operation::Join {
            left: input_of(left.0),
            right: input_of(right.0),
            left_on: vec![ColumnIndex(left.1)],
            right_on: vec![ColumnIndex(right.1)],
            output: output
                .iter()
                .map(|(side, column)| JoinColumn {
                    side: *side,
                    column: ColumnIndex(*column),
                })
                .collect(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        left.0,
        right.0,
    )
}

/// The counted grouping of `from` by its refinement, counting the distinct names when `distinct`.
pub(crate) fn group_count(from: PlanInput, distinct: bool) -> Step {
    Step {
        operation: Operation::GroupCount {
            from: input_of(from),
            keys: vec![ColumnIndex(2)],
            distinct: distinct.then_some(ColumnIndex(3)),
        },
        inputs: vec![from],
    }
}
