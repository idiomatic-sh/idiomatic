//! The whole batch is checked before any row is delivered.
use super::types::{RenderAnswer, RenderedOutput};
use crate::domain::contract::types::PortableOperatorFault;
use ryo_values::domain::rows::types::RowIndex;

/// Refuses a batch holding an unsupported answer, before any delivery; delivers the known texts in order.
pub(crate) fn deliver(
    answers: &[RenderAnswer<'_>],
    each: &mut dyn RenderedOutput,
) -> Result<(), PortableOperatorFault> {
    if answers.contains(&RenderAnswer::Unsupported) {
        return Err(PortableOperatorFault::UnsupportedRender);
    }
    for (position, answer) in answers.iter().enumerate() {
        if let RenderAnswer::Known(text) = answer {
            each.text(RowIndex(position), *text)
                .map_err(|_| PortableOperatorFault::Interrupted)?;
        }
    }
    Ok(())
}

/// Renders the node of every candidate row in one batch and emits each candidate whose node has a known
/// rendering, with the rendering appended; a batch holding an unsupported answer emits nothing.
pub(crate) fn evaluate(
    node: ryo_values::domain::plan_parts::types::ColumnIndex,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    scratch: &mut crate::domain::contract::types::RowScratch,
) -> Result<(), PortableOperatorFault> {
    use crate::domain::contract::types::RowRead;
    use ryo_values::domain::plan_parts::types::InputNumber;
    let crate::domain::contract::types::RowScratch {
        row,
        other,
        text,
        slots,
        nodes,
        rendered,
        ..
    } = scratch;
    let candidates = driver.population(InputNumber(0))?;
    nodes.clear();
    for at in 0..candidates.0 {
        if driver.population_row(InputNumber(0), RowIndex(at), row) != RowRead::Read {
            return Err(PortableOperatorFault::NotRun);
        }
        let ryo_relations::domain::relations::types::Value::Node(found) =
            ryo_values::domain::rows::service::column_in(row, node)
                .copied()
                .ok_or(
                    crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                        index: node,
                    },
                )?
        else {
            return Err(PortableOperatorFault::ShapeMismatch);
        };
        nodes.push(found);
    }
    if nodes.is_empty() {
        return Ok(());
    }
    driver.render(nodes)?;
    text.0.clear();
    slots.0.clear();
    rendered.clear();
    for at in 0..nodes.len() {
        let start = text.0.len();
        rendered.push(driver.rendered(RowIndex(at), text));
        slots.0.push(start..text.0.len());
    }
    let answers: Vec<RenderAnswer<'_>> = rendered
        .iter()
        .zip(slots.0.iter())
        .map(|(kind, place)| match kind {
            crate::domain::contract::types::RenderKind::Known => {
                RenderAnswer::Known(ryo_values::domain::row_texts::types::CellText(
                    text.0.get(place.clone()).unwrap_or_default(),
                ))
            }
            crate::domain::contract::types::RenderKind::NotApplicable => {
                RenderAnswer::NotApplicable
            }
            crate::domain::contract::types::RenderKind::Unsupported => RenderAnswer::Unsupported,
        })
        .collect();
    let mut minting = crate::domain::render_batch::types::RenderMinting {
        driver,
        row,
        joined: other,
        outcome: Ok(()),
    };
    let delivered = deliver(&answers, &mut minting);
    minting.outcome.and(delivered)
}

/// The one lowering of a render: its node column and the role the node is rendered in, for the `render` kernel.
pub(crate) fn lower(
    node: ryo_values::domain::plan_parts::types::ColumnIndex,
    context: ryo_language::domain::semantics::types::RenderContext,
) -> super::types::RenderParts {
    super::types::RenderParts { node, context }
}
