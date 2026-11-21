//! The services of the `graphs` part: `follow` as the `FixedPoints` graph table over its edges' rows. The table's row is the
//! contract's (`lowered::graphs::service::graph_row`): the keys' cells in the keys' order, then a source, then a fixed point
//! it reaches, which is the plan's own row of the step.

use crate::domain::code::records::types::EdgeColumns;
use crate::domain::lowered::types::{Emptiness, FillInput, GraphFill, MadeFill};
use crate::domain::lowering::definitions::service::fresh_cells;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::{copied, derived};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, RowColumn, RowsPerFile,
};
use crate::domain::lowering::tables::fills::service::{column_list, fill_of, made_once, scanned};
use crate::domain::lowering::tables::inputs::service::{input, rule};
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening};
use ryo_values::domain::plan_parts::types::{Input, StepId};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::table_words::types::ColumnCount;

/// The branch of a `follow`: one row of its graph table. The edges are a definition's rows, or a relation's own, scanned
/// as they stand; the table is made once for the step's node.
pub(crate) fn follow_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::Follow {
        edges,
        keys,
        source,
        destination,
    } = operation
    else {
        return None;
    };
    let shape = rule(state)?.shapes.get(usize::from(step))?.clone()?;
    let read = input(state, drafts, *edges, InputWant::Defined)?;
    let rows = match (&read.emptiness, edges) {
        (Emptiness::Definition(definition), _) => AtomRows::Definition(*definition),
        (_, Input::Relation(relation)) => AtomRows::Relation(*relation),
        (_, Input::Step(_) | Input::BoundObservation(_)) => return None,
    };
    let edges = FillInput {
        pipelines: [scanned(drafts, rows)].into(),
        keys: column_list(keys.iter().map(|key| RowColumn(usize::from(*key)))),
        order: None,
    };
    let told = EdgeColumns {
        source: made_column(RowColumn(usize::from(*source)))?,
        destination: made_column(RowColumn(usize::from(*destination)))?,
    };
    let fill = MadeFill::Graph(GraphFill::FixedPoints(told), [edges].into());
    let of = fill_of(state, step, copied(&read.emptiness))?;
    let table = made_once(drafts, of, fill, ColumnCount(shape.0.len()));
    let cells = fresh_cells(drafts, &shape);
    Some(Flattened {
        branches: vec![FlatBranch {
            atoms: vec![FlatAtom::Rows {
                rows: AtomRows::Made(table),
                cells: cells.clone(),
                per_file: RowsPerFile::Many,
            }],
            row: cells,
        }],
        emptiness: derived(operation, vec![read.emptiness]),
        keyless: Vec::new(),
    })
}
