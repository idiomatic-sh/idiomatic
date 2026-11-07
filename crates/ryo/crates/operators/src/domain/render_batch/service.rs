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

/// Interns each known rendering as derived text and emits its candidate row with it appended.
struct RenderMinting<'a> {
    driver: &'a mut dyn crate::domain::evaluation::types::InvocationDriver,
    row: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    joined: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    failure: Option<PortableOperatorFault>,
}

impl RenderedOutput for RenderMinting<'_> {
    fn text(
        &mut self,
        position: RowIndex,
        text: ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError> {
        use ryo_values::domain::plan_parts::types::InputNumber;
        if self
            .driver
            .population_row(InputNumber(0), position, self.row)
            != crate::domain::evaluation::types::RowRead::Read
        {
            self.failure = Some(PortableOperatorFault::NotRun);
            return Err(crate::domain::contract::errors::ContractError::Interrupted);
        }
        let made = self.driver.mint(text);
        self.joined.clear();
        self.joined.extend_from_slice(self.row);
        self.joined.push(made);
        self.driver
            .emit(self.joined)
            .map_err(|_| crate::domain::contract::errors::ContractError::Interrupted)
    }
}

/// Renders the node of every candidate row in one batch and emits each candidate whose node has a known
/// rendering, with the rendering appended; a batch holding an unsupported answer emits nothing.
pub(crate) fn evaluate(
    node: ryo_values::domain::plan_parts::types::ColumnIndex,
    driver: &mut dyn crate::domain::evaluation::types::InvocationDriver,
    scratch: &mut crate::domain::evaluation::types::EvaluationScratch,
) -> Result<(), PortableOperatorFault> {
    use crate::domain::evaluation::types::RowRead;
    use ryo_values::domain::plan_parts::types::InputNumber;
    let crate::domain::evaluation::types::EvaluationScratch {
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
            crate::domain::evaluation::service::cell(row, node)?
        else {
            return Err(PortableOperatorFault::ShapeMismatch);
        };
        nodes.push(found);
    }
    if nodes.is_empty() {
        return Ok(());
    }
    driver.render(nodes)?;
    text.clear();
    slots.clear();
    rendered.clear();
    for at in 0..nodes.len() {
        let start = text.len();
        rendered.push(driver.rendered(RowIndex(at), text));
        slots.push(start..text.len());
    }
    let answers: Vec<RenderAnswer<'_>> = rendered
        .iter()
        .zip(slots.iter())
        .map(|(kind, place)| match kind {
            super::types::RenderKind::Known => {
                RenderAnswer::Known(ryo_values::domain::row_texts::types::CellText(
                    text.get(place.clone()).unwrap_or_default(),
                ))
            }
            super::types::RenderKind::NotApplicable => RenderAnswer::NotApplicable,
            super::types::RenderKind::Unsupported => RenderAnswer::Unsupported,
        })
        .collect();
    let mut minting = RenderMinting {
        driver,
        row,
        joined: other,
        failure: None,
    };
    let delivered = deliver(&answers, &mut minting);
    minting.failure.map_or(delivered, Err)
}
