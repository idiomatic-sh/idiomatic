//! The entities one graph walk has met, each once, kept with their room between walks and emptied at the cost of what the
//! walk met, not of the room the largest walk before it left.

use crate::domain::physical_plan::types::keyed_rows::value::IndexKeyHash;
use crate::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::Value;
use std::hash::{Hash as _, Hasher as _};

/// The entities one walk has met. Every walk kernel marks what it meets here and resets it before its next start.
#[derive(Debug)]
pub(crate) struct VisitedSet {
    /// The entities met, by their hash.
    table: hashbrown::HashTable<Value>,
    /// The entities met, in the order met, so a reset removes only these.
    met: Vec<Value>,
    /// Spike (ryo-jenl): the walk that last met each node of `file`, by the node's place in the file: a node is a dense id,
    /// so it is met by position, never hashed.
    stamps: Vec<u32>,
    /// Spike (ryo-jenl): the walks reset so far; the open walk's stamp is one more.
    walks: u32,
    /// Spike (ryo-jenl): the file whose nodes are met by position (`u64::MAX`: none yet).
    file: u64,
    /// Spike (ryo-jenl): whether the open walk met a node by position.
    placed: bool,
}

/// Spike (ryo-jenl): the most places a visited set addresses by position.
const MOST_PLACES: u64 = 1 << 20;


impl VisitedSet {
    /// Marks `entity` met, and answers whether it was met already in this walk.
    pub(crate) fn meet(&mut self, entity: Value) -> SetMembership {
        if let Value::Node(node) = entity
            && crate::spike::visited_on()
        {
            let (file, place) = (node.0 >> 32, node.0 & 0xffff_ffff);
            if file != self.file && !self.placed {
                self.file = file;
            }
            if file == self.file && place < MOST_PLACES {
                let place = usize::try_from(place).unwrap_or(usize::MAX);
                if place >= self.stamps.len() {
                    self.stamps.resize(place + 1, 0);
                }
                let stamp = self.walks.wrapping_add(1);
                if let Some(slot) = self.stamps.get_mut(place) {
                    self.placed = true;
                    if *slot == stamp {
                        return SetMembership::In;
                    }
                    *slot = stamp;
                    return SetMembership::Out;
                }
            }
        }
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
        if self.placed {
            self.walks = self.walks.wrapping_add(1);
            if self.walks.wrapping_add(1) == 0 {
                self.stamps.fill(0);
                self.walks = 0;
            }
            self.placed = false;
        }
        if self.met.len().saturating_mul(8) >= self.table.capacity() {
            self.table.clear();
        } else {
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
        self.stamps.kept();
        self.walks = 0;
        self.placed = false;
    }

}

impl Default for VisitedSet {
    fn default() -> Self {
        Self {
            table: hashbrown::HashTable::new(),
            met: Vec::new(),
            stamps: Vec::new(),
            walks: 0,
            file: u64::MAX,
            placed: false,
        }
    }
}
