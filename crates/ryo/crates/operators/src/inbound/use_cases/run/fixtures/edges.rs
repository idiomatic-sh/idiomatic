//! The edge rows the graph kinds' run cases read, and their answers as the cases state them.
use super::{TableInputs, TextResources, emitted};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value as Cell};
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the cases read edges from: parent, child, ordinal, field, file.
pub(crate) const EDGES: Relation = Relation::SemanticChild;

/// The node at position `at` of file 0.
pub(crate) fn node(at: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(at)))
}

/// The edge rows `parent -> child`, each with its ordinal, the field `f` and file 0.
pub(crate) fn edges(pairs: &[(u32, u32, i64)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(EDGES).len());
    for (parent, child, ordinal) in pairs {
        cells.extend([
            node(*parent),
            node(*child),
            Cell::Number(Integer(*ordinal)),
            texts.symbol("f"),
            Cell::File(FileId(0)),
        ]);
    }
    Table {
        kinds: kinds(EDGES).to_vec(),
        cells,
        count: RowCount(pairs.len()),
    }
}

/// A cell as the case states it: a node by its position, a number as itself.
pub(crate) fn stated(cell: Cell) -> i64 {
    if let Cell::Node(id) = cell {
        i64::from(ryo_relations::domain::relations::service::node_position(id).0)
    } else if let Cell::Number(Integer(number)) = cell {
        number
    } else {
        panic!("a case states nodes and numbers only, not {cell:?}")
    }
}

/// The rows `step` answers over `pairs` as the edges, each cell as the case states it.
///
/// # Errors
///
/// The fault `plan` or `run` answers.
pub(crate) fn edge_rows(
    step: Step,
    pairs: &[(u32, u32, i64)],
) -> Result<Vec<i64>, PortableOperatorFault> {
    let mut resources = TextResources::new(&["f"]);
    let table = edges(pairs, &resources);
    let inputs = TableInputs::new(&[(EDGES, &table)]);
    let rows = emitted(
        &[step],
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )?;
    Ok(rows
        .iter()
        .flat_map(|table| table.cells.iter().copied().map(stated))
        .collect())
}
