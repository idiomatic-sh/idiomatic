//! Invocation scratch is moved out under a short borrow, never held behind a
//! scheduler RefMut across semantic or downstream callbacks. Publications and
//! their accounting stay with the canonical scope/relation owner.
use crate::domain::{
    file_program::types::NodeId,
    operating::{errors::OperatingError, types::Room},
};
use std::cell::RefCell;

struct Slot {
    running: bool,
    room: Option<Box<Room>>,
}
/// One scratch collection owned by the canonical scope. No dependency scheduling,
/// input data, publication aliases, budget or lifetime ledger lives here.
pub(crate) struct InvocationRooms {
    slots: RefCell<Vec<Slot>>,
}
impl InvocationRooms {
    pub(crate) fn new(nodes: usize) -> Self {
        Self {
            slots: RefCell::new(
                (0..nodes)
                    .map(|_| Slot {
                        running: false,
                        room: None,
                    })
                    .collect(),
            ),
        }
    }
    pub(crate) fn take(&self, node: NodeId) -> Result<InvocationRoom<'_>, OperatingError> {
        let mut slots = self.slots.borrow_mut();
        let slot = slots.get_mut(node.0).ok_or(OperatingError::NotRun)?;
        if slot.running {
            return Err(OperatingError::NotRun);
        }
        slot.running = true;
        let room = slot.room.take().unwrap_or_default();
        // No Ref/RefMut is stored in the invocation. Another node may be demanded
        // reentrantly while the caller uses this room or returns an error.
        drop(slots);
        Ok(InvocationRoom {
            owner: self,
            node,
            room: Some(room),
        })
    }
}
pub(crate) struct InvocationRoom<'a> {
    owner: &'a InvocationRooms,
    node: NodeId,
    room: Option<Box<Room>>,
}
impl InvocationRoom<'_> {
    pub(crate) fn room(&mut self) -> Result<&mut Room, OperatingError> {
        self.room.as_deref_mut().ok_or(OperatingError::NotRun)
    }
}
impl Drop for InvocationRoom<'_> {
    fn drop(&mut self) {
        let mut slots = self.owner.slots.borrow_mut();
        if let Some(slot) = slots.get_mut(self.node.0) {
            slot.room = self.room.take();
            slot.running = false;
        }
    }
}

#[cfg(test)]
#[path = "rooms/service_test.rs"]
mod tests;
