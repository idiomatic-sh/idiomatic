//! A worker's scratch buffer as the end of a file meets it, and the room it keeps.
use super::constants::{KEPT_ROOM, POOL_TABLE_KINDS};
use super::service::{kept_list, kept_queue, kept_table};
use ryo_values::domain::rows::types::{RowCount, Table};

pub(crate) mod kept_pool;


/// A worker's scratch buffer at the end of a file: emptied, and its room dropped when it is past `KEPT_ROOM`.
pub(crate) trait KeptRoom {
    /// Empties the buffer, keeping its room only when it is within `KEPT_ROOM`.
    fn kept(&mut self);

}

impl<T> KeptRoom for Vec<T> {
    fn kept(&mut self) {
        kept_list(self);
    }

}

impl<T> KeptRoom for hashbrown::HashTable<T> {
    fn kept(&mut self) {
        kept_table(self);
    }

}

impl<T> KeptRoom for std::collections::VecDeque<T> {
    fn kept(&mut self) {
        kept_queue(self);
    }

}

/// Whether an emptied buffer's room is within the caps a pool keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoomFit {
    /// Within every cap: the pool may keep it.
    Fits,
    /// Past a cap: it is dropped.
    Exceeds,
}

/// How many emptied buffers a pool keeps, or keeps at most.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PoolSize(pub(crate) usize);

/// Whether a buffer holds any room at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoomHeld {
    /// It holds room: a build can fill it without taking a pooled one.
    Holds,
    /// It holds none: a build takes a pooled one first, and a pool keeps none of it.
    Empty,
}

/// A buffer a worker's pool keeps between its uses: emptied as it is given back, and kept only when it holds room that fits.
pub(crate) trait Poolable: Default {
    /// Whether its room is within the caps a pool keeps.
    fn fits(&self) -> RoomFit;
    /// Whether it holds any room.
    fn held(&self) -> RoomHeld;
    /// Empties it, keeping its room.
    fn emptied(&mut self);
}

/// A node table a worker's pool keeps: within the room any scratch keeps past a file, and within `POOL_TABLE_KINDS` kinds.
impl Poolable for Table {
    fn fits(&self) -> RoomFit {
        if self.cells.capacity() <= KEPT_ROOM && self.kinds.capacity() <= POOL_TABLE_KINDS {
            RoomFit::Fits
        } else {
            RoomFit::Exceeds
        }
    }

    fn held(&self) -> RoomHeld {
        if self.cells.capacity() == 0 && self.kinds.capacity() == 0 {
            RoomHeld::Empty
        } else {
            RoomHeld::Holds
        }
    }

    fn emptied(&mut self) {
        self.cells.clear();
        self.kinds.clear();
        self.count = RowCount(0);
    }
}
