//! The emptied buffers of one kind a worker keeps for its next uses: at most a count of them, each within the room a pool
//! keeps, so the room a worker keeps does not grow with how many buffers a plan uses.
use super::super::{PoolSize, Poolable, RoomFit, RoomHeld};

/// The emptied buffers of one kind a worker keeps for its next uses.
#[derive(Debug, Default)]
pub(crate) struct KeptPool<T> {
    /// The buffers kept, each empty and each holding room.
    kept: Vec<T>,
}

impl<T: Poolable> KeptPool<T> {
    /// A buffer of no rows: one the pool kept, or a new one.
    pub(crate) fn fresh(&mut self) -> T {
        self.kept.pop().unwrap_or_default()
    }

    /// Keeps `item` emptied for a later use. One that holds no room, one past a cap, or one more than `most` is dropped.
    pub(crate) fn pooled(&mut self, mut item: T, most: PoolSize) {
        if item.held() == RoomHeld::Empty
            || item.fits() == RoomFit::Exceeds
            || PoolSize(self.kept.len()) >= most
        {
            return;
        }
        item.emptied();
        self.kept.push(item);
    }

    /// Readies `item` for a build about to fill it: emptied in place when it holds room, else given a kept buffer's room.
    pub(crate) fn refilled(&mut self, item: &mut T) {
        match item.held() {
            RoomHeld::Holds => item.emptied(),
            RoomHeld::Empty => *item = self.fresh(),
        }
    }


}
