//! Test-only observations of actual owner backing and selection state.
use crate::application::file_dag::derived::owner::*;
use ryo_relations::domain::relations::types::Relation;
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::application::file_dag) struct IndexObservation {
    pub available: bool,
}
impl DerivedOwner {
    pub fn scoped_observation(
        &self,
    ) -> (usize, Weak<DerivedStorage>) {
        (
            self.next_scope.get().0,
            self.last_scoped_storage.borrow().clone(),
        )
    }

    pub fn resource_fixture_counts(
        &self,
    ) -> crate::application::file_dag::derived::owner::fixtures::resources::Snapshot {
        let backend = self.backend.borrow();
        let cache = backend.as_ref().expect("configured actual owner");
        crate::application::file_dag::derived::owner::fixtures::resources::Snapshot {
            graph_builds: cache.graph_counts.builds.0,
            graph_reuses: cache.graph_counts.reuses.0,
            graph_early_stops: cache.graph_counts.early_stops.0,
        }
    }
    pub fn index_observation(&self) -> IndexObservation {
        self.backend
            .borrow()
            .as_ref()
            .map_or_else(IndexObservation::default, |_| IndexObservation {
                available: true,
            })
    }
    pub(in crate::application::file_dag) fn relation_eligible(&self, relation: Relation) -> bool {
        self.backend
            .borrow()
            .as_ref()
            .is_some_and(|cache| cache.eligible.contains(&relation))
    }

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

    pub(in crate::application::file_dag::derived::owner) fn first_index_weak(
        &self,
    ) -> Option<Weak<ryo_derived::domain::equality::types::KeyIndex>> {
        self.state.borrow().entries.values().find_map(|entry| {
            if entry._plan.kind != DerivedKind::Equality {
                return None;
            }
            let storage = entry.weak.upgrade()?;
            storage
                .payload()
                .share_equality()
                .map(|index| Arc::downgrade(&index.0))
        })
    }
    pub(in crate::application::file_dag) fn shared_index_backings(&self) -> usize {
        self.ledger
            .borrow()
            .backing_count(crate::application::file_dag::derived::ledger::types::BackingTag(11))
            .0
    }
    pub fn publication_ownership_bytes(
        &self,
    ) -> (usize, usize, usize) {
        let state = self.ledger.borrow().snapshot();
        (
            state.required_bytes.0,
            state.optional_bytes.0,
            state.unique_bytes.0,
        )
    }
    /// The observation follows one qualified producer's admitted populations.
    /// Ambiguous matching requirements are refused rather than picking a view.
    pub(in crate::application::file_dag) fn population_view_witness(
        &self,
        populations: &[PopulationId],
        kind: DerivedKind,
    ) -> Result<
        Option<crate::application::file_dag::derived::owner::witness::service::BackingWitness>,
        OperatingError,
    > {
        let state = self.state.borrow();
        let mut matching = state.entries.iter().filter(|(key, entry)| {
            entry._plan.kind == kind
                && key.populations[..usize::from(key.len.0)]
                    .iter()
                    .any(|population| populations.contains(population))
        });
        let witness = matching.next().map(|(_, entry)| {
            crate::application::file_dag::derived::owner::witness::service::BackingWitness::new(
                entry.weak.clone(),
            )
        });
        if matching.next().is_some() {
            return Err(OperatingError::WrongOperation);
        }
        Ok(witness)
    }
    pub fn view_witness(
        &self,
        kind: DerivedKind,
    ) -> Option<crate::application::file_dag::derived::owner::witness::service::BackingWitness>
    {
        self.state
            .borrow()
            .entries
            .values()
            .find(|entry| entry._plan.kind == kind)
            .map(|entry| {
                crate::application::file_dag::derived::owner::witness::service::BackingWitness::new(
                    entry.weak.clone(),
                )
            })
    }
}
