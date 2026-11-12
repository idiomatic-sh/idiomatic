//! Test-only observations of actual owner backing and selection state.
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage;
use crate::application::file_dag::derived::owner::service::*;
use crate::application::file_dag::derived::population::service::PopulationId;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use std::sync::{Arc, Weak};
/// What a proof observes of the owner's invocation indexes.
#[derive(Clone, Copy, Debug, Default)]
pub struct IndexObservation {
    pub available: bool,
}
impl DerivedOwner {
    /// The scoped builds counted, and the storage of the last one.
    pub fn scoped_observation(&self) -> (usize, Weak<DerivedStorage>) {
        (
            self.next_scope.get().0,
            self.last_scoped_storage.borrow().clone(),
        )
    }

    /// The counts the resource fixture reads.
    ///
    /// # Panics
    ///
    /// When the owner has no installed backend: the fixture reads a configured owner only.
    #[expect(
        clippy::expect_used,
        reason = "a fixture reads the counts of an owner its proof configured: none is a broken setup that fails the proof"
    )]
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
    /// What the owner's index acquisitions observed.
    pub fn index_observation(&self) -> IndexObservation {
        self.backend
            .borrow()
            .as_ref()
            .map_or_else(IndexObservation::default, |_| IndexObservation {
                available: true,
            })
    }
    /// Whether `relation` is eligible for a retained index.
    pub fn relation_eligible(&self, relation: Relation) -> bool {
        self.backend
            .borrow()
            .as_ref()
            .is_some_and(|cache| cache.eligible.contains(&relation))
    }

    /// Whether no invocation holds an index handle.
    pub fn no_invocation_index_handles(&self) -> bool {
        self.state.borrow().entries.values().all(|entry| {
            if entry._plan.kind != DerivedKind::Equality {
                return true;
            }
            let owned =
                usize::from(entry.retained.is_some()) + usize::from(entry.required.is_some());
            entry.weak.strong_count() == owned
        })
    }

    /// A weak reference to the first retained index, to observe its release.
    pub fn first_index_weak(&self) -> Option<Weak<ryo_derived::domain::equality::types::KeyIndex>> {
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
    /// How many index backings are shared.
    pub fn shared_index_backings(&self) -> usize {
        self.ledger
            .borrow()
            .backing_count(crate::application::file_dag::accounting::ledger::types::BackingTag(11))
            .0
    }
    /// The publication, ownership and retained bytes the owner accounts for.
    pub fn publication_ownership_bytes(&self) -> (usize, usize, usize) {
        let state = self.ledger.borrow().snapshot();
        (
            state.required_bytes.0,
            state.optional_bytes.0,
            state.unique_bytes.0,
        )
    }
    /// The observation follows one qualified producer's admitted populations.
    /// Ambiguous matching requirements are refused rather than picking a view.
    ///
    /// # Errors
    ///
    /// [`OperatingError::WrongOperation`] when more than one retained view of `kind` matches the populations.
    pub fn population_view_witness(
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
    /// The backing witness of the retained view of `kind`, when one is retained.
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
