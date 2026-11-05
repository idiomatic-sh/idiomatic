//! Immutable policy answers and diagnostic values; no backend reference escapes.
use super::*;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::application::file_dag) struct IndexObservation {
    pub available: bool,
    pub builds: u64,
    pub reuses: u64,
}
impl DerivedOwner {
    pub(in crate::application::file_dag) fn selector_work(
        &self,
    ) -> Result<crate::domain::operator::types::RowsFed, OperatingError> {
        let backend = self.backend.borrow();
        let backend = backend.as_ref().ok_or(OperatingError::NotRun)?;
        Ok(crate::domain::operator::types::RowsFed([
            backend.selection_counts.predicate_rows,
            0,
            0,
        ]))
    }

    #[cfg(test)]
    pub(crate) fn index_fixture_state(&self) -> (u64, u64, usize, bool) {
        self.backend
            .borrow()
            .as_ref()
            .map_or((0, 0, 0, true), |cache| {
                (
                    cache.builds,
                    cache.reuses,
                    self.retained_index_count(),
                    self.no_invocation_index_handles(),
                )
            })
    }
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
        self.backend.borrow().as_ref().is_some_and(|cache| {
            self.has_live_equality(rows, columns)
                || super::super::indexes::can_retain(cache, relation, rows, columns)
        })
    }
    pub(super) fn has_live_equality(&self, rows: ColumnsView<'_>, columns: &[ColumnIndex]) -> bool {
        let count = rows.columns.first().map_or(0, |column| column.0.len());
        self.state.borrow().entries.values().any(|entry| {
            matches!(&entry._plan.request, ViewRequest::Equality { columns: requested } if requested == columns)
                && entry._plan.schemas.first().is_some_and(|schema| schema == rows.kinds)
                && entry.weak.upgrade().is_some_and(|storage| {
                    matches!(storage.view(), Ok(ryo_derived::domain::contract::types::DerivedView::Equality(index)) if index.rows().len() == count)
                })
        })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn retained_index_count(&self) -> usize {
        self.state
            .borrow()
            .entries
            .values()
            .filter(|entry| entry._plan.kind == DerivedKind::Equality && entry.retained.is_some())
            .count()
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn selection_counts(&self) -> (u64, u64) {
        self.backend.borrow().as_ref().map_or((0, 0), |cache| {
            (cache.selection_counts.builds, cache.selection_counts.reuses)
        })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn retained_predicate_rows(&self) -> Option<usize> {
        self.state.borrow().entries.values().find_map(|entry| {
            if entry._plan.kind != DerivedKind::PredicateMatches {
                return None;
            }
            let storage = &entry.retained.as_ref()?.storage;
            match storage.view().ok()? {
                ryo_derived::domain::contract::types::DerivedView::RowIds(rows) => Some(rows.len()),
                _ => None,
            }
        })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn no_invocation_index_handles(&self) -> bool {
        self.state.borrow().entries.values().all(|entry| {
            if entry._plan.kind != DerivedKind::Equality {
                return true;
            }
            let owned =
                usize::from(entry.retained.is_some()) + usize::from(entry.required.is_some());
            entry.weak.strong_count() == owned
        })
    }

    #[cfg(test)]
    pub(super) fn first_index_weak(
        &self,
    ) -> Option<Weak<ryo_derived::domain::contract::types::KeyIndex>> {
        self.state.borrow().entries.values().find_map(|entry| {
            if entry._plan.kind != DerivedKind::Equality {
                return None;
            }
            let storage = entry.weak.upgrade()?;
            storage
                .payload()
                .share_equality()
                .map(|index| Arc::downgrade(&index))
        })
    }
    #[cfg(test)]
    pub(super) fn first_index_owners(&self) -> Option<usize> {
        self.first_index_weak().map(|index| index.strong_count())
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn shared_index_backings(&self) -> usize {
        self.ledger.borrow().backing_count(11)
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn publication_ownership_bytes(
        &self,
    ) -> (usize, usize, usize) {
        let state = self.ledger.borrow().snapshot();
        (
            state.required_bytes,
            state.node_optional_bytes,
            state.unique_bytes,
        )
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn view_weak(
        &self,
        kind: DerivedKind,
    ) -> Option<std::sync::Weak<DerivedStorage>> {
        self.state
            .borrow()
            .entries
            .values()
            .find(|entry| entry._plan.kind == kind)
            .map(|entry| entry.weak.clone())
    }
}
