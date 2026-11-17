//! The entities one graph walk has met, each once, kept with their room between walks and emptied at the cost of what the
//! walk met, not of the room the largest walk before it left.

use crate::domain::physical_plan::types::keyed_rows::value::IndexKeyHash;
use crate::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::Value;
use std::hash::{Hash as _, Hasher as _};

/// The entities one walk has met. Every walk kernel marks what it meets here and resets it before its next start.
#[derive(Debug, Default)]
pub(crate) struct VisitedSet {
    /// The entities met, by their hash.
    table: hashbrown::HashTable<Value>,
    /// The entities met, in the order met, so a reset removes only these.
    met: Vec<Value>,
    /// The table slots every reset so far has emptied.
    #[cfg(test)]
    work: ResetWork,
}

/// How many table slots the resets of a visited set have emptied.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResetWork(pub(crate) usize);

impl VisitedSet {
    /// Marks `entity` met, and answers whether it was met already in this walk.
    pub(crate) fn meet(&mut self, entity: Value) -> SetMembership {
        let hash = value_hash(entity).0;
        if self.table.find(hash, |held| *held == entity).is_some() {
            return SetMembership::In;
        }
        let _met = self
            .table
            .insert_unique(hash, entity, |held| value_hash(*held).0);
        self.met.push(entity);
        SetMembership::Out
    }

    /// Empties it for the next walk: removes each entity the walk met, or, when it met an eighth of the table's room or
    /// more, empties the whole table, whichever touches fewer slots.
    pub(crate) fn reset(&mut self) {
        if self.met.len().saturating_mul(8) >= self.table.capacity() {
            #[cfg(test)]
            {
                self.work.0 += self.table.capacity();
            }
            self.table.clear();
        } else {
            #[cfg(test)]
            {
                self.work.0 += self.met.len();
            }
            for entity in self.met.iter().copied() {
                if let Ok(held) = self
                    .table
                    .find_entry(value_hash(entity).0, |held| *held == entity)
                {
                    let _removed = held.remove();
                }
            }
        }
        self.met.clear();
    }

    /// Whether no entity is met.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> crate::domain::contract::types::PredicateOutcome {
        if self.table.is_empty() && self.met.is_empty() {
            crate::domain::contract::types::PredicateOutcome::Holds
        } else {
            crate::domain::contract::types::PredicateOutcome::Fails
        }
    }

    /// The table slots every reset so far has emptied.
    #[cfg(test)]
    pub(crate) fn reset_work(&self) -> ResetWork {
        self.work
    }
}

/// The hash a walk keeps an entity it met under.
pub(crate) fn value_hash(value: Value) -> IndexKeyHash {
    let mut hasher = rustc_hash::FxHasher::default();
    value.hash(&mut hasher);
    IndexKeyHash(hasher.finish())
}

impl crate::domain::kept_room::types::KeptRoom for VisitedSet {
    fn kept(&mut self) {
        self.table.kept();
        self.met.kept();
    }

    #[cfg(test)]
    fn room(&self) -> crate::domain::kept_room::types::Room {
        self.table.room().max(self.met.room())
    }
}
