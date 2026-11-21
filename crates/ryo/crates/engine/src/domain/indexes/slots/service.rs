//! The services of the `slots` part: an index's table among the entries, and its slot kept as its site's.

use crate::domain::code::types::IndexSite;
use crate::domain::indexes::types::{IndexArena, IndexSlot, TableWord};

/// The table of an index among the entries.
#[inline]
pub(crate) fn table_of(entries: &mut [TableWord], slot: IndexSlot) -> &mut [TableWord] {
    entries
        .get_mut(slot.table.0..slot.table.0.saturating_add(slot.words.0))
        .unwrap_or_default()
}

/// Keeps `slot` as the index of `site`.
pub(crate) fn kept(arena: &mut IndexArena, site: IndexSite, slot: IndexSlot) {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if arena.slots.len() <= place {
        arena
            .slots
            .resize(place.saturating_add(1), IndexSlot::default());
    }
    if let Some(held) = arena.slots.get_mut(place) {
        *held = slot;
    }
}
