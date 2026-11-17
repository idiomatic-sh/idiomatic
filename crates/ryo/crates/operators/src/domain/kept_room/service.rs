//! The rule of the room kept, for each kind of buffer a worker's scratch is made of: one past the cap is dropped, one within
//! it is emptied and keeps its room.
use super::constants::KEPT_ROOM;

/// Empties `list`, keeping its room only when it is within `KEPT_ROOM`.
pub(crate) fn kept_list<T>(list: &mut Vec<T>) {
    if list.capacity() > KEPT_ROOM {
        *list = Vec::new();
    } else {
        list.clear();
    }
}

/// Empties `table`, keeping its room only when it is within `KEPT_ROOM`.
pub(crate) fn kept_table<T>(table: &mut hashbrown::HashTable<T>) {
    if table.capacity() > KEPT_ROOM {
        *table = hashbrown::HashTable::new();
    } else {
        table.clear();
    }
}

/// Empties `queue`, keeping its room only when it is within `KEPT_ROOM`.
pub(crate) fn kept_queue<T>(queue: &mut std::collections::VecDeque<T>) {
    if queue.capacity() > KEPT_ROOM {
        *queue = std::collections::VecDeque::new();
    } else {
        queue.clear();
    }
}
