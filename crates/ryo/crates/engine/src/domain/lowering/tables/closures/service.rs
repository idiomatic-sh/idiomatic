//! The services of the `closures` part: the branch of a closure, and the edges a closure or a nearest walk reads.

use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::definitions::service::fresh_variable;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::copied;
use crate::domain::lowering::emptiness::service::derived;
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::service::picked;
use crate::domain::lowering::flattening::types::EdgeColumns;
use crate::domain::lowering::flattening::types::EdgeRows;
use crate::domain::lowering::flattening::types::{FlatAtom, FlatTerm, RowColumn};
use crate::domain::lowering::nodes::service::input_shape;
use crate::domain::lowering::tables::inputs::service::{input, rule};
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Operation;

/// Where a walk reads its edges: a step's rows are its definition; a relation's own rows are read in place, one row of the
/// relation in cells of the walk's own.
pub(crate) fn edge_rows(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    edges: Input,
) -> Option<EdgeRows> {
    let read = input(state, drafts, edges, InputWant::Defined)?;
    match (read.emptiness, edges) {
        (Emptiness::Definition(definition), _) => Some(EdgeRows::Definition(definition)),
        (_, Input::Relation(_)) => {
            let FlatAtom::Rows {
                rows,
                cells,
                per_file,
            } = read.branches.into_iter().next()?.atoms.into_iter().next()?
            else {
                return None;
            };
            Some(EdgeRows::Read {
                rows,
                cells,
                per_file,
            })
        }
        (_, Input::Step(_) | Input::BoundObservation(_)) => None,
    }
}

/// The branch of a closure: each row of the edges gives a source and its keys, and one walk reaches from that source
/// over one or more edges of the same keys. The edges are a definition; the walk asks it by a source, or by a destination
/// when it is entered from what is reached.
pub(crate) fn closure_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::Closure {
        edges,
        keys,
        source,
        destination,
    } = operation
    else {
        return None;
    };
    let shape = input_shape(&rule(state)?.shapes, *edges)?;
    let mut read = input(state, drafts, *edges, InputWant::Defined)?;
    let rows = edge_rows(state, drafts, *edges)?;
    let edges_empty = copied(&read.emptiness);
    let column = |place: &ColumnIndex| RowColumn(usize::from(*place));
    // The file's own column is no key within the file: the walk stays in the other keys.
    let within = keys
        .iter()
        .filter(|key| shape.0.get(usize::from(**key)) != Some(&CellKind::File));
    let walked = EdgeColumns {
        rows,
        keys: within
            .clone()
            .filter_map(|key| made_column(column(key)))
            .collect(),
        source: made_column(column(source))?,
        destination: made_column(column(destination))?,
    };
    for branch in &mut read.branches {
        let reached = FlatTerm::Variable(fresh_variable(drafts));
        let from = *branch.row.get(usize::from(*source))?;
        let stays = picked(&branch.row, within.clone().map(column))?;
        let mut row = picked(&branch.row, keys.iter().map(column))?;
        row.extend([from, reached]);
        branch.atoms.push(FlatAtom::Closure {
            edges: walked.clone(),
            keys: stays,
            source: from,
            reached,
        });
        branch.row = row;
    }
    Some(Flattened {
        emptiness: derived(operation, vec![edges_empty]),
        ..read
    })
}
