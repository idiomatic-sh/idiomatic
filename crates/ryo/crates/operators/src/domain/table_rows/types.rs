//! The engine-owned delivery of each row's texts.
use crate::domain::contract::errors::ContractError;
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
    fn row(&mut self, texts: RowTexts<'_>) -> Result<(), ContractError>;
}

/// Interns each delivered text as derived text and emits the row.
pub(crate) struct RowMinting<'a> {
    /// The engine's driver, which interns each text and takes each row.
    pub driver: &'a mut dyn crate::domain::contract::types::RowDriver,
    /// The row being made.
    pub row: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
}

impl crate::domain::table_rows::types::TableRowOutput for RowMinting<'_> {
    fn row(
        &mut self,
        texts: crate::domain::table_rows::types::RowTexts<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError> {
        self.row.clear();
        for text in texts.0 {
            let made = self.driver.mint(*text);
            self.row.push(made);
        }
        self.driver
            .emit(self.row)
            .map_err(|_| crate::domain::contract::errors::ContractError::Interrupted)
    }
}
