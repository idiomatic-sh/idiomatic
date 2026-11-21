//! The services of the `indexes` concept: an index built once per file over a source's rows, every bound column as one
//! key. One scheme for every source and every width of key: an open table of the distinct keys, each with the stretch of
//! rows that hold it. An index is named by its site and by what it holds, never by where its key list lies in memory.

use super::constants::{EMPTY_RUN, LEAST_ENTRIES, PLACED_SPAN, RUN_START_SHIFT};
use super::reading::service::{cell_at, start_of};
use super::types::{
    Addressing, ColumnView, FileStamp, HashShift, IndexArena, IndexBuilt, IndexSlot, KeyBounds,
    KeyNumber, SourceRow, StretchPlace, TableWord,
};
use crate::domain::code::types::IndexSite;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Forgets every index of the last file, keeping the room.
pub(crate) fn clear_indexes(arena: &mut IndexArena) {
    arena.entries.clear();
    arena.rows.clear();
}

/// Whether the index of `site` was built for the file `stamp` names.
#[inline]
pub(crate) fn state_of(arena: &IndexArena, site: IndexSite, stamp: FileStamp) -> IndexBuilt {
    match arena
        .slots
        .get(usize::try_from(site.0).unwrap_or(usize::MAX))
    {
        Some(slot) if slot.stamp == stamp => IndexBuilt::Built,
        Some(_) | None => IndexBuilt::Unbuilt,
    }
}

/// Starts an index over a source of `rows` rows by a key of `width` cells: `key_column` follows once for each cell of the
/// key, then `finish`.
pub(crate) fn begin(arena: &mut IndexArena, rows: Count, width: Count) {
    arena.keys.clear();
    arena
        .keys
        .resize(rows.0.saturating_mul(width.0), TableWord(0));
    arena.width = width;
    arena.pending = rows;
    arena.base = SourceRow(0);
    arena.bounds = KeyBounds {
        least: CellWord(u64::MAX),
        greatest: CellWord(0),
    };
}

/// Says, of the index `begin` started, that its source's first row is numbered `base`: the rows it holds are numbered
/// from there, as the handles of that source are.
pub(crate) fn based(arena: &mut IndexArena, base: SourceRow) {
    arena.base = base;
}

/// Writes one cell of each row's key from a column's cells, and answers the bounds widened by them.
#[inline]
fn keys_written<'k>(
    keys: impl Iterator<Item = &'k mut TableWord>,
    cells: impl Iterator<Item = &'k CellWord>,
    bounds: KeyBounds,
) -> KeyBounds {
    let mut widened = bounds;
    for (key, cell) in keys.zip(cells) {
        *key = TableWord(cell.0);
        widened.least = widened.least.min(*cell);
        widened.greatest = widened.greatest.max(*cell);
    }
    widened
}

/// Gives the index being built the column its key's cell `at` is read from: one pass that writes it into each row's key.
/// The one loop is compiled for a key of one cell and for a wider one, and for a column that lies cell after cell and for
/// one that lies in a table's rows, so none of the four walks by a stride it does not have.
pub(crate) fn key_column(arena: &mut IndexArena, at: Count, view: ColumnView<'_>) {
    let cells = view.cells.get(view.first.0..).unwrap_or_default();
    let (width, stride) = (arena.width.0.max(1), view.stride.0.max(1));
    let keys = &mut arena.keys;
    let bounds = match (width, stride) {
        (1, 1) => keys_written(keys.iter_mut(), cells.iter(), arena.bounds),
        (1, _) => keys_written(keys.iter_mut(), cells.iter().step_by(stride), arena.bounds),
        (_, 1) => keys_written(
            keys.chunks_exact_mut(width)
                .filter_map(|row| row.get_mut(at.0)),
            cells.iter(),
            arena.bounds,
        ),
        (_, _) => keys_written(
            keys.chunks_exact_mut(width)
                .filter_map(|row| row.get_mut(at.0)),
            cells.iter().step_by(stride),
            arena.bounds,
        ),
    };
    if at.0 == 0 {
        arena.bounds = bounds;
    }
}

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
    let Some(start) = start_of(
        slot.addressing,
        slot.shift,
        slot.mask,
        row.iter().map(|cell| CellWord(cell.0)),
    ) else {
        return KeyNumber(0);
    };
    let mut at = start.0;
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

/// The table of an index among the entries.
#[inline]
fn table_of(entries: &mut [TableWord], slot: IndexSlot) -> &mut [TableWord] {
    let length = slot
        .mask
        .0
        .saturating_add(1)
        .saturating_mul(slot.width.0.saturating_add(1));
    entries
        .get_mut(slot.table.0..slot.table.0.saturating_add(length))
        .unwrap_or_default()
}

/// The three passes of a build. Each row finds its key and the key counts it; each key's count becomes where its stretch
/// starts, and its entry is given the stretch; each row is written at its key's next place. Keys are numbered as first
/// seen, so the counts and the places of a source whose every row has its own key are read and written in order, and only
/// the table is reached out of order. Rows are visited in order, so a stretch holds its rows ascending.
fn grouped(arena: &mut IndexArena, slot: IndexSlot) {
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
    let mut next = 0_u32;
    for count in groups.iter_mut() {
        let start = next;
        next = next.wrapping_add(count.0);
        count.0 = start;
    }
    for entry in table.chunks_exact_mut(stride) {
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

/// The last pass, of an ordered index alone: each key's rows sorted by the order column, then by row.
fn sorted(arena: &mut IndexArena, slot: IndexSlot, order: ColumnView<'_>) {
    let IndexArena { entries, rows, .. } = arena;
    let stride = slot.width.0.saturating_add(1);
    let stretches = rows.get_mut(slot.first.0..).unwrap_or_default();
    for entry in table_of(entries, slot).chunks_exact(stride) {
        if let Some(run) = entry.last()
            && *run != EMPTY_RUN
        {
            let found = super::reading::service::run_from(*run);
            if let Some(stretch) = stretches.get_mut(found.start.0..found.end.0)
                && stretch.len() > 1
            {
                stretch.sort_unstable_by_key(|row| {
                    (
                        cell_at(order, Count(usize::try_from(row.0).unwrap_or(usize::MAX))),
                        *row,
                    )
                });
            }
        }
    }
}

/// Ends the index `begin` started, as the index of `site` for the file `stamp` names; with `order`, each key's rows are
/// sorted by that column. A key of one cell whose cells lie close together is placed by the cell itself.
pub(crate) fn finish(
    arena: &mut IndexArena,
    site: IndexSite,
    stamp: FileStamp,
    order: Option<ColumnView<'_>>,
) {
    let rows = arena.pending.0;
    let span = usize::try_from(arena.bounds.greatest.0.saturating_sub(arena.bounds.least.0))
        .unwrap_or(usize::MAX);
    let placed = arena.width.0 == 1
        && rows > 0
        && span
            < rows
                .saturating_mul(PLACED_SPAN.0)
                .saturating_add(LEAST_ENTRIES.0);
    let entries = if placed {
        span.saturating_add(1)
    } else {
        rows.saturating_add(rows / 2)
    }
    .max(LEAST_ENTRIES.0)
    .next_power_of_two();
    let slot = IndexSlot {
        stamp,
        table: Count(arena.entries.len()),
        mask: Count(entries.saturating_sub(1)),
        shift: HashShift(u64::BITS.saturating_sub(entries.trailing_zeros())),
        addressing: if placed {
            Addressing::Placed(arena.bounds.least)
        } else {
            Addressing::Hashed
        },
        width: arena.width,
        first: Count(arena.rows.len()),
        count: Count(rows),
    };
    let stride = slot.width.0.saturating_add(1);
    arena.entries.resize(
        slot.table.0.saturating_add(entries.saturating_mul(stride)),
        EMPTY_RUN,
    );
    arena
        .rows
        .resize(slot.first.0.saturating_add(rows), SourceRow(0));
    grouped(arena, slot);
    if let Some(column) = order {
        sorted(arena, slot, column);
    }
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if arena.slots.len() <= place {
        arena
            .slots
            .resize(place.saturating_add(1), IndexSlot::default());
    }
    if let Some(kept) = arena.slots.get_mut(place) {
        *kept = slot;
    }
}
