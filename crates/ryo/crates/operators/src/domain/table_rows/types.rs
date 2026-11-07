//! The engine-owned delivery of each row's texts.
use crate::domain::contract::types::SemanticInterrupted;
use ryo_values::domain::row_texts::types::CellText;

/// The texts of one row, in the order of the requested keys.
#[derive(Clone, Copy, Debug)]
pub struct RowTexts<'a>(pub &'a [CellText<'a>]);

/// Receives the texts of one row, in the order of the requested keys; the engine interns and publishes them.
pub trait TableRowOutput {
    /// One row's texts.
    ///
    /// # Errors
    ///
    /// The engine retained an error that stops the delivery.
    fn row(&mut self, texts: RowTexts<'_>) -> Result<(), SemanticInterrupted>;
}
