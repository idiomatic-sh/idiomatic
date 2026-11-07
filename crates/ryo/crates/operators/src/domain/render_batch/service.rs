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
