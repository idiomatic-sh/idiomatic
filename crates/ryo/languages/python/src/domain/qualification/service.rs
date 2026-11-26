//! Lexical import and builtin origins. Dynamic values are unknown, never guessed from spelling.
use super::constants::{ADDED_BUILTINS, BUILTIN_NAMES, BUILTIN_PLACES, BUILTIN_SLOTS, PYTHON_THREE};
use super::types::{Version};

use ryo::languages::{NameText};


/// Availability from the same canonical inventory for source reads and occurrence-free catalogs.
pub(crate) fn builtin_named(name: NameText<'_>) -> Option<Version> {
    is_builtin(name).then(|| {
        ADDED_BUILTINS
            .iter()
            .find(|entry| entry.name.0 == name.0)
            .map_or(PYTHON_THREE, |entry| entry.since)
    })
}
/// The hash of a builtin name's text (64-bit FNV), the same when the crate is compiled and when a name is looked up.
const fn text_hash(text: &[u8]) -> usize {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut at = 0;
    while at < text.len() {
        hash = (hash ^ text[at] as u64).wrapping_mul(0x0000_0100_0000_01b3);
        at += 1;
    }
    (hash ^ (hash >> 32)) as usize
}

/// The lookup of `names` by the hash of their text (`BUILTIN_SLOTS`): each name at the first empty place from its hash's.
pub(crate) const fn slotted(names: &[NameText<'static>]) -> [u16; BUILTIN_PLACES] {
    let mut slots = [0_u16; BUILTIN_PLACES];
    let mut number = 0;
    while number < names.len() {
        let mut place = text_hash(names[number].0.as_bytes()) % BUILTIN_PLACES;
        while slots[place] != 0 {
            place = (place + 1) % BUILTIN_PLACES;
        }
        slots[place] = (number + 1) as u16;
        number += 1;
    }
    slots
}

/// Whether `name` is one of `BUILTIN_NAMES`, by the lookup built from them.
fn is_builtin(name: NameText<'_>) -> bool {
    let mut place = text_hash(name.0.as_bytes()) % BUILTIN_PLACES;
    loop {
        let Some(slot) = BUILTIN_SLOTS.get(place).copied() else {
            return false;
        };
        let Some(number) = usize::from(slot).checked_sub(1) else {
            return false;
        };
        if BUILTIN_NAMES.get(number).is_some_and(|entry| entry.0 == name.0) {
            return true;
        }
        place = place.wrapping_add(1) % BUILTIN_PLACES;
    }
}
