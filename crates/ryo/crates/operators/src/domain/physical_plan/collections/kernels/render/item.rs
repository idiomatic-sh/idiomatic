//! The `render` item of the `kernels` collection: a blocking kernel that asks the engine once for the renderings of every
//! row's node, each in the operation's role, and keeps each row whose node has a known rendering, with it appended; one
//! unsupported answer refuses the batch before any row.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{count_of, kinds_of, row_of};
use crate::domain::contract::types::PortableOperatorFault;
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::physical_plan::types::{InputRows, RenderRequests, Resources};
use crate::domain::render_batch::types::RenderParts;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `render`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct RenderItem;

#[cfg(test)]
impl KernelsItem for RenderItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        if matches!(lowering, Lowering::Render { .. }) {
            Reach::Runs
        } else {
            Reach::Skips
        }
    }
}

/// Reads every row's node, asks `resources` once for their renderings in `render`'s role, and writes into `output` each row
/// whose node has a known rendering, with its text minted and appended, in row order. No row asks nothing.
///
/// # Errors
///
/// `NoSuchColumn` or `ShapeMismatch` for the first row whose node column is past it or holds no node, before any rendering
/// is asked for; the engine's refusal; `UnsupportedRender` when any rendering is unsupported, before any row is written.
pub(crate) fn rendered(
    render: &RenderParts,
    input: InputRows<'_>,
    resources: &mut dyn Resources,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    output.kinds = kinds_of(input).to_vec();
    output.kinds.push(CellKind::Symbol);
    let mut row = Vec::new();
    let mut requests = Vec::with_capacity(count_of(input).0);
    for at in 0..count_of(input).0 {
        let cells = row_of(input, RowIndex(at), &mut row);
        let Value::Node(node) = ryo_values::domain::rows::service::column_in(cells, render.node)
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: render.node })?
        else {
            return Err(PortableOperatorFault::ShapeMismatch);
        };
        requests.push(RenderRequest {
            node,
            context: render.context,
        });
    }
    if requests.is_empty() {
        return Ok(());
    }
    let outcomes = resources.render(RenderRequests(&requests))?;
    if outcomes
        .0
        .iter()
        .any(|outcome| matches!(outcome, RenderOutcome::Unsupported(_)))
    {
        return Err(PortableOperatorFault::UnsupportedRender);
    }
    for (at, outcome) in outcomes.0.iter().enumerate() {
        if let RenderOutcome::Known(text) = outcome {
            let made = resources.mint(CellText(&text.0));
            output
                .cells
                .extend_from_slice(row_of(input, RowIndex(at), &mut row));
            output.cells.push(made);
            output.count = RowCount(output.count.0 + 1);
        }
    }
    Ok(())
}
