//! The services of the `hashing` part: an open table of the distinct keys, each with the stretch of rows that hold it.
//! Any cells, any width of key.

use crate::domain::code::types::IndexSite;
use crate::domain::indexes::constants::{EMPTY_RUN, LEAST_ENTRIES, RUN_START_SHIFT};
use crate::domain::indexes::reading::service::start_of;
use crate::domain::indexes::slots::service::{kept, table_of};
use crate::domain::indexes::types::{
    Addressing, FileStamp, HashShift, IndexArena, IndexSlot, KeyNumber, SourceRow, StretchPlace,
    TableWord,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The number of a row's key among the keys in the order first seen: its entry is found, or an empty one taken and the key
/// numbered, and the key counts the row. A key is compared and written cell by cell, in line: a key is a cell or two, and a
/// call would cost more than the cells.
#[inline]
fn entered(
    table: &mut [TableWord],
    groups: &mut Vec<StretchPlace>,
    slot: IndexSlot,
    row: &[TableWord],
) -> KeyNumber {
    let stride = slot.width.0.saturating_add(1);
    let mut at = start_of(slot.shift, row.iter().map(|cell| CellWord(cell.0))).0;
    loop {
        let from = at.wrapping_mul(stride);
        let Some((run, key)) = table
            .get_mut(from..from.wrapping_add(stride))
            .and_then(<[TableWord]>::split_last_mut)
        else {
            return KeyNumber(0);
        };
        if *run == EMPTY_RUN {
            for (cell, own) in key.iter_mut().zip(row) {
                *cell = *own;
            }
            let number = u32::try_from(groups.len()).unwrap_or(u32::MAX);
            *run = TableWord(u64::from(number));
            groups.push(StretchPlace(1));
            return KeyNumber(number);
        }
        if key.iter().zip(row).all(|(cell, own)| cell == own) {
            let number = u32::try_from(run.0).unwrap_or(u32::MAX);
            if let Some(count) = groups.get_mut(usize::try_from(number).unwrap_or(usize::MAX)) {
                count.0 = count.0.wrapping_add(1);
            }
            return KeyNumber(number);
        }
        at = at.wrapping_add(1) & slot.mask.0;
    }
}

/// Each key's count becomes where its stretch starts, and its entry is given the stretch: the pass between counting a
/// source's rows by key and placing them.
fn stretched(table: &mut [TableWord], groups: &mut [StretchPlace], stride: Count) {
    let mut next = 0_u32;
    for count in groups.iter_mut() {
        let start = next;
        next = next.wrapping_add(count.0);
        count.0 = start;
    }
    for entry in table.chunks_exact_mut(stride.0) {
        if let Some(run) = entry.last_mut()
            && *run != EMPTY_RUN
        {
            let number = usize::try_from(run.0).unwrap_or(usize::MAX);
            let start = groups.get(number).map_or(0, |place| place.0);
            let end = groups
                .get(number.saturating_add(1))
                .map_or(next, |place| place.0);
            *run = TableWord((u64::from(start) << RUN_START_SHIFT.0) | u64::from(end));
        }
    }
}

/// The three passes of a build. Each row finds its key and the key counts it; each key's count becomes where its stretch
/// starts, and its entry is given the stretch; each row is written at its key's next place. Keys are numbered as first
/// seen, so the counts and the places of a source whose every row has its own key are read and written in order, and only
/// the table is reached out of order. Rows are visited in order, so a stretch holds its rows ascending.
pub(crate) fn grouped(arena: &mut IndexArena, slot: IndexSlot) {
    let IndexArena {
        entries,
        rows,
        places,
        groups,
        keys,
        base,
        ..
    } = arena;
    let stride = slot.width.0.saturating_add(1);
    let table = table_of(entries, slot);
    places.clear();
    groups.clear();
    if slot.width.0 == 0 {
        places.extend((0..slot.count.0).map(|_| entered(table, groups, slot, &[])));
    } else {
        places.extend(
            keys.chunks_exact(slot.width.0)
                .map(|row| entered(table, groups, slot, row)),
        );
    }
    stretched(table, groups, Count(stride));
    let stretches = rows.get_mut(slot.first.0..).unwrap_or_default();
    for (row, key) in places.iter().enumerate() {
        if let Some(cursor) = groups.get_mut(usize::try_from(key.0).unwrap_or(usize::MAX)) {
            if let Some(cell) = stretches.get_mut(usize::try_from(cursor.0).unwrap_or(usize::MAX)) {
                *cell = SourceRow(base.0.wrapping_add(u32::try_from(row).unwrap_or(u32::MAX)));
            }
            cursor.0 = cursor.0.wrapping_add(1);
        }
    }
}

/// Builds, as the index of `site` for the file `stamp` names, the index of a source's `rows` rows by a key the source
/// holds as a small code, the key's cell being the word `word_of` names for the code: `codes` gives, each time it is
/// asked, every row's code, row after row; the source's first row is numbered nought. The rows are counted by code and
/// placed by code, two passes over the column of codes; a key is entered in the open table once for each code that
/// occurs, not once for each row, and two codes of one word share its stretch. The rows of a stretch are ascending.
pub(crate) fn coded<K: Iterator<Item = Count>>(
    arena: &mut IndexArena,
    site: IndexSite,
    stamp: FileStamp,
    rows: Count,
    word_of: impl Fn(Count) -> CellWord,
    codes: impl Fn() -> K,
) {
    // Rows that follow one another with the same code are counted in hand, and written when the code changes: a
    // count read back for every row would wait on the write before it.
    arena.counts.clear();
    let (mut last, mut run) = (Count(usize::MAX), 0_u32);
    for code in codes().chain(std::iter::once(Count(usize::MAX))) {
        if code == last {
            run = run.wrapping_add(1);
        } else {
            if run != 0 {
                if arena.counts.len() <= last.0 {
                    arena
                        .counts
                        .resize(last.0.saturating_add(1), StretchPlace(0));
                }
                if let Some(count) = arena.counts.get_mut(last.0) {
                    count.0 = count.0.wrapping_add(run);
                }
            }
            last = code;
            run = 1;
        }
    }
    let present = arena.counts.iter().filter(|count| count.0 != 0).count();
    let entries = present
        .saturating_add(present / 2)
        .max(LEAST_ENTRIES.0)
        .next_power_of_two();
    let slot = IndexSlot {
        stamp,
        table: Count(arena.entries.len()),
        words: Count(entries.saturating_mul(2)),
        mask: Count(entries.saturating_sub(1)),
        shift: HashShift(u64::BITS.saturating_sub(entries.trailing_zeros())),
        addressing: Addressing::Hashed,
        width: Count(1),
        first: Count(arena.rows.len()),
        count: rows,
    };
    arena
        .entries
        .resize(slot.table.0.saturating_add(slot.words.0), EMPTY_RUN);
    arena
        .rows
        .resize(slot.first.0.saturating_add(rows.0), SourceRow(0));
    let IndexArena {
        entries: all,
        rows: held,
        places,
        groups,
        counts,
        ..
    } = arena;
    let table = table_of(all, slot);
    places.clear();
    groups.clear();
    // Each code that occurs enters its word once, and the word's key counts the code's rows.
    places.extend(counts.iter().enumerate().map(|(code, count)| {
        if count.0 == 0 {
            return KeyNumber(u32::MAX);
        }
        let key = entered(table, groups, slot, &[TableWord(word_of(Count(code)).0)]);
        if let Some(group) = groups.get_mut(usize::try_from(key.0).unwrap_or(usize::MAX)) {
            group.0 = group.0.wrapping_add(count.0.wrapping_sub(1));
        }
        key
    }));
    stretched(table, groups, Count(2));
    let stretches = held.get_mut(slot.first.0..).unwrap_or_default();
    // The same in the second pass: a key's next place is held in hand while the code stays.
    let no_place = u32::MAX;
    let (mut last, mut key, mut cursor) = (Count(usize::MAX), usize::MAX, no_place);
    let mut row = 0_u32;
    for code in codes() {
        if code != last {
            if cursor != no_place
                && let Some(held) = groups.get_mut(key)
            {
                held.0 = cursor;
            }
            last = code;
            key = places.get(code.0).map_or(usize::MAX, |named| {
                usize::try_from(named.0).unwrap_or(usize::MAX)
            });
            cursor = groups.get(key).map_or(no_place, |held| held.0);
        }
        if cursor != no_place {
            if let Some(cell) = stretches.get_mut(usize::try_from(cursor).unwrap_or(usize::MAX)) {
                *cell = SourceRow(row);
            }
            cursor = cursor.wrapping_add(1);
        }
        row = row.wrapping_add(1);
    }
    kept(arena, site, slot);
}
