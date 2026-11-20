//! Spike (ryo-fmx3, host3): the room of a file's rule records, kept on the worker's thread across files, so the next file
//! writes its records into memory the last one warmed instead of into fresh pages.

use crate::application::file_dag::program::file_session::types::RanRule;
use std::cell::RefCell;

thread_local! {
    static ROOM: RefCell<Vec<RanRule>> = const { RefCell::new(Vec::new()) };
}

/// The room for `count` rule records: what the last file on this thread gave back, emptied, or a new list.
pub(crate) fn rule_room(count: usize) -> Vec<RanRule> {
    let mut room = ROOM.with(|room| std::mem::take(&mut *room.borrow_mut()));
    room.clear();
    room.reserve(count);
    room
}

/// Gives back a file's rule records, read and emptied by the caller, for the next file on this thread.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::file_session::service::given_back_rules;
/// given_back_rules(Vec::new());
/// ```
pub fn given_back_rules(mut rules: Vec<RanRule>) {
    rules.clear();
    ROOM.with(|room| {
        let mut room = room.borrow_mut();
        if rules.capacity() > room.capacity() {
            *room = rules;
        }
    });
}
