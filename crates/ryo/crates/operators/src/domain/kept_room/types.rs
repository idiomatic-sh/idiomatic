//! A worker's scratch buffer as the end of a file meets it, and the room it keeps.
use super::service::{kept_list, kept_queue, kept_table};

/// The room a buffer keeps, in items.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Room(pub(crate) usize);

/// A worker's scratch buffer at the end of a file: emptied, and its room dropped when it is past `KEPT_ROOM`.
pub(crate) trait KeptRoom {
    /// Empties the buffer, keeping its room only when it is within `KEPT_ROOM`.
    fn kept(&mut self);

    /// The room it keeps.
    #[cfg(test)]
    fn room(&self) -> Room;
}

impl<T> KeptRoom for Vec<T> {
    fn kept(&mut self) {
        kept_list(self);
    }

    #[cfg(test)]
    fn room(&self) -> Room {
        Room(self.capacity())
    }
}

impl<T> KeptRoom for hashbrown::HashTable<T> {
    fn kept(&mut self) {
        kept_table(self);
    }

    #[cfg(test)]
    fn room(&self) -> Room {
        Room(self.capacity())
    }
}

impl<T> KeptRoom for std::collections::VecDeque<T> {
    fn kept(&mut self) {
        kept_queue(self);
    }

    #[cfg(test)]
    fn room(&self) -> Room {
        Room(self.capacity())
    }
}
