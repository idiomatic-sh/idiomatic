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
    #[cfg(test)]
    pub(crate) fn index_fixture_state(&self) -> (u64, u64, usize, bool) {
        self.backend
            .borrow()
            .as_ref()
            .map_or((0, 0, 0, true), |cache| {
                (
                    cache.builds,
                    cache.reuses,
                    cache.entries.len(),
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
        self.backend
            .borrow()
            .as_ref()
            .is_some_and(|cache| super::super::indexes::can_retain(cache, relation, rows, columns))
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
            if let DerivedStorage::RowIds { rows, .. } = storage.as_ref() {
                Some(rows.len())
            } else {
                None
            }
        })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn no_invocation_index_handles(&self) -> bool {
        let backend = self.backend.borrow();
        let Some(cache) = backend.as_ref() else {
            return true;
        };
        let state = self.state.borrow();
        for cached in &cache.entries {
            let mut owned = 1;
            let mut seen = Vec::new();
            for entry in state.entries.values() {
                let Some(storage) = entry.weak.upgrade() else {
                    continue;
                };
                let DerivedStorage::EqualityShared(index) = storage.as_ref() else {
                    continue;
                };
                if !Arc::ptr_eq(index, &cached.index) {
                    continue;
                }
                let references =
                    usize::from(entry.retained.is_some()) + usize::from(entry.required.is_some());
                if Arc::strong_count(&storage) != references + 1 {
                    return false;
                }
                let address = Arc::as_ptr(&storage) as usize;
                if !seen.contains(&address) {
                    seen.push(address);
                    owned += 1;
                }
            }
            if Arc::strong_count(&cached.index) != owned {
                return false;
            }
        }
        true
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
