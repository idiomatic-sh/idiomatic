//! The `names` service of the model: free functions named by verb over the names of one file, with no IO and no ports.

use super::constants::{HASH_START, HASH_STEP, UNKNOWN_NAME};
use super::types::{NameCode, NameEntry, NameHash, NameSlot, NameTable};
use crate::domain::qualification::service::builtin_named;
use crate::domain::qualification::types::Version;
use ryo::languages::{NameText, Offset};

/// Forgets every name, keeping the buffers.
pub(crate) fn clear(table: &mut NameTable) {
    table.texts.0.clear();
    table.entries.clear();
    table.slots.fill(NameSlot(0));
}

/// The hash of `text`.
fn hashed(text: NameText<'_>) -> NameHash {
    let mut hash = HASH_START.0;
    for byte in text.0.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(HASH_STEP.0);
    }
    NameHash(hash)
}

/// The text of the entry `entry`.
fn entry_text<'a>(table: &'a NameTable, entry: &NameEntry) -> NameText<'a> {
    let start = usize::try_from(entry.start.0).unwrap_or(usize::MAX);
    let end = usize::try_from(entry.end.0).unwrap_or(usize::MAX);
    NameText(table.texts.0.get(start..end).unwrap_or_default())
}

/// The code `text` has in the table, when it is a name of the file.
#[must_use]
pub(crate) fn find(table: &NameTable, text: NameText<'_>) -> Option<NameCode> {
    let mask = table.slots.len().checked_sub(1)?;
    let hash = hashed(text);
    let mut place = usize::try_from(hash.0).unwrap_or(usize::MAX) & mask;
    loop {
        let slot = table.slots.get(place)?;
        let code = NameCode(slot.0.checked_sub(1)?);
        let entry = table.entries.get(usize::from(code))?;
        if entry.hash == hash && entry_text(table, entry) == text {
            return Some(code);
        }
        place = place.wrapping_add(1) & mask;
    }
}

/// Places the entry `code` in the first free slot from its hash on.
fn place(table: &mut NameTable, code: NameCode, hash: NameHash) {
    let Some(mask) = table.slots.len().checked_sub(1) else {
        return;
    };
    let mut place = usize::try_from(hash.0).unwrap_or(usize::MAX) & mask;
    while let Some(slot) = table.slots.get_mut(place) {
        if slot.0 == 0 {
            *slot = NameSlot(code.0.saturating_add(1));
            return;
        }
        place = place.wrapping_add(1) & mask;
    }
}

/// Doubles the slots when the entries fill half of them, and places every entry again.
fn grow(table: &mut NameTable) {
    if table.entries.len().saturating_mul(2) < table.slots.len() {
        return;
    }
    let wanted = table.slots.len().saturating_mul(2).max(64);
    table.slots.clear();
    table.slots.resize(wanted, NameSlot(0));
    for number in 0..table.entries.len() {
        if let Some(entry) = table.entries.get(number).copied() {
            place(
                table,
                NameCode(u32::try_from(number).unwrap_or(u32::MAX)),
                entry.hash,
            );
        }
    }
}

/// The code of `text`, added to the table when the file has not named it yet.
pub(crate) fn intern(table: &mut NameTable, text: NameText<'_>) -> NameCode {
    if let Some(code) = find(table, text) {
        return code;
    }
    grow(table);
    let code = NameCode(u32::try_from(table.entries.len()).unwrap_or(u32::MAX));
    let start = Offset(u32::try_from(table.texts.0.len()).unwrap_or(u32::MAX));
    table.texts.0.push_str(text.0);
    let hash = hashed(text);
    table.entries.push(NameEntry {
        start,
        end: Offset(u32::try_from(table.texts.0.len()).unwrap_or(u32::MAX)),
        hash,
        builtin: builtin_named(text),
    });
    place(table, code, hash);
    code
}

/// The text of the name `code`; none for the code of no name and for the unknown names of a wildcard import.
#[must_use]
pub(crate) fn text_of(table: &NameTable, code: NameCode) -> Option<NameText<'_>> {
    if code >= UNKNOWN_NAME {
        return None;
    }
    table
        .entries
        .get(usize::from(code))
        .map(|entry| entry_text(table, entry))
}

/// The language version whose builtins first have the name `code`, when they have it.
#[must_use]
pub(crate) fn builtin_of(table: &NameTable, code: NameCode) -> Option<Version> {
    table
        .entries
        .get(usize::from(code))
        .and_then(|entry| entry.builtin)
}
