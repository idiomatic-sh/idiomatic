//! Borrowed ordered match contracts; no engine authority.
/// Borrowed access to already prepared ordered keys and row delivery.
pub trait OrderedMatchDriver {
    fn left_len(&self) -> usize;
    fn right_len(&self) -> usize;
    fn compare_keys(&self, left: usize, right: usize) -> std::cmp::Ordering;
    fn right_order_before_or_equal(&self, left: usize, right: usize) -> bool;
    /// Read both admitted rows before calling the projection; the original read order is observable.
    fn visit_pair(
        &mut self,
        left: usize,
        right: usize,
        project: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), PortableOperatorFault>,
    ) -> Result<(), PortableOperatorFault>;
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), SemanticInterrupted>;
}

use crate::domain::contract::types::{PortableOperatorFault, SemanticInterrupted};
