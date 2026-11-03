//! Immutable policy answers and diagnostic values; no backend reference escapes.
use super::*;
use ryo_language::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::application::file_dag) struct IndexObservation {
    pub available: bool,
    pub builds: u64,
    pub reuses: u64,
}
impl DerivedOwner {
    pub(in crate::application::file_dag) fn index_observation(&self) -> IndexObservation {
        self.backend
            .borrow()
            .as_ref()
            .map_or_else(IndexObservation::default, |cache| IndexObservation {
                available: true,
                builds: cache.builds,
                reuses: cache.reuses,
            })
    }
    pub(in crate::application::file_dag) fn relation_eligible(&self, relation: Relation) -> bool {
        self.backend
            .borrow()
            .as_ref()
            .is_some_and(|cache| cache.eligible.contains(&relation))
    }
    pub(in crate::application::file_dag) fn can_retain_equality(
        &self,
        relation: Relation,
        rows: ColumnsView<'_>,
        columns: &[ColumnIndex],
    ) -> bool {
        self.backend
            .borrow()
            .as_ref()
            .is_some_and(|cache| super::super::indexes::can_retain(cache, relation, rows, columns))
    }
}
