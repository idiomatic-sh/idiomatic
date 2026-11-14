//! Borrowed ordered match contracts; no engine authority.
/// Whether a right row of equal key sits at or before a left row in their original order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyOrder {
    /// The right row is before the left row, or at the same place.
    BeforeOrEqual,
    /// The right row is after the left row.
    After,
}

/// Borrowed access to already prepared ordered keys and row delivery.
pub trait OrderedMatchDriver {
    fn left_len(&self) -> ryo_values::domain::rows::types::RowCount;
    fn right_len(&self) -> ryo_values::domain::rows::types::RowCount;
    fn compare_keys(
        &self,
        left: ryo_values::domain::rows::types::RowIndex,
        right: ryo_values::domain::rows::types::RowIndex,
    ) -> std::cmp::Ordering;
    fn right_order_before_or_equal(
        &self,
        left: ryo_values::domain::rows::types::RowIndex,
        right: ryo_values::domain::rows::types::RowIndex,
    ) -> KeyOrder;
    /// Read both admitted rows before calling the projection; the original read order is observable.
    ///
    /// # Errors
    ///
    /// The [`PortableOperatorFault`] `project` returns, or one the engine raises reading either row.
    fn visit_pair(
        &mut self,
        left: ryo_values::domain::rows::types::RowIndex,
        right: ryo_values::domain::rows::types::RowIndex,
        project: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), PortableOperatorFault>,
    ) -> Result<(), PortableOperatorFault>;
    /// # Errors
    ///
    /// [`ContractError::Interrupted`] when an engine-retained error interrupts the delivery.
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), ContractError>;
}

use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::PortableOperatorFault;

/// An engine driver that lends the ordered keys of both inputs.
pub trait OrderedMatchEvaluation: crate::domain::contract::types::RowDriver {
    /// The engine's prepared ordered keys of both inputs.
    ///
    /// # Errors
    ///
    /// The engine retained an error: an input or the ordered keys are absent.
    fn ordered_match(
        &mut self,
    ) -> Result<&mut dyn OrderedMatchDriver, crate::domain::contract::types::PortableOperatorFault>;
}

/// What an ordered match lowers to: both sides' keys and orders and the output columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OrderedParts {
    /// The left keys.
    pub(crate) left_on: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The right keys.
    pub(crate) right_on: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The left order column.
    pub(crate) left_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The right order column.
    pub(crate) right_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The output columns.
    pub(crate) output: Vec<ryo_values::domain::plan_parts::types::JoinColumn>,
}
