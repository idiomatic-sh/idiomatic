//! Test-only observations of actual owner backing and selection state.
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
}
