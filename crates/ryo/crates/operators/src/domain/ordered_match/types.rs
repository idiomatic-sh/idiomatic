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
    fn compare_keys(&self, left: ryo_values::domain::rows::types::RowIndex, right: ryo_values::domain::rows::types::RowIndex) -> std::cmp::Ordering;
    fn right_order_before_or_equal(&self, left: ryo_values::domain::rows::types::RowIndex, right: ryo_values::domain::rows::types::RowIndex) -> KeyOrder;
    /// Read both admitted rows before calling the projection; the original read order is observable.
    fn visit_pair(
        &mut self,
        left: ryo_values::domain::rows::types::RowIndex,
        right: ryo_values::domain::rows::types::RowIndex,
        project: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), PortableOperatorFault>,
    ) -> Result<(), PortableOperatorFault>;
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), ContractError>;
}

use crate::domain::contract::types::{PortableOperatorFault};
use crate::domain::contract::errors::ContractError;
