//! The services of the `indexes` concept: an index built once per file over a source's rows, every bound column as one
//! key. An index has one form for its readers, each key's rows side by side in the arena; how a key finds its stretch,
//! and so how the index is built, follows the shape of the key: an open table of the distinct keys for any cells of any
//! width; a table of places addressed by the cell itself, filled by counting, for one column of cells that lie close
//! together; and nothing at all for the rows of a source by their own id. An index is named by its site and by what it
//! holds, never by where its key list lies in memory.

use super::constants::{EMPTY_RUN, LEAST_ENTRIES};
use super::counting::service::{cells_lie, counted, counted_slot};
use super::hashing::service::grouped;
use super::reading::service::{cell_at, runs};
use super::slots::service::{kept, table_of};
use super::types::{
    Addressing, ColumnView, FileStamp, HashShift, IndexArena, IndexBuilt, IndexRead, IndexSlot,
    KeyBounds, KeyCells, SourceRow, TableWord,
};
use crate::domain::code::types::IndexSite;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Forgets every index of the last file, keeping the room, and the rows at the front that hold their own number: those
/// are grown here, between files, to what the largest index of rows by their own id asked for.
pub(crate) fn clear_indexes(arena: &mut IndexArena) {
    arena.entries.clear();
    arena.rows.truncate(arena.own.0);
    if arena.wanted.0 > arena.own.0 {
        let (from, to) = (
            u32::try_from(arena.own.0).unwrap_or(u32::MAX),
            u32::try_from(arena.wanted.0).unwrap_or(u32::MAX),
        );
        arena.rows.extend((from..to).map(SourceRow));
        arena.own = Count(arena.rows.len());
    }
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

/// The last pass, of an ordered index alone: each key's rows sorted by the order column, then by row.
fn sorted(arena: &mut IndexArena, slot: IndexSlot, order: ColumnView<'_>) {
    let IndexArena { entries, rows, .. } = arena;
    let stretches = rows.get_mut(slot.first.0..).unwrap_or_default();
    let table = IndexRead {
        entries: table_of(entries, slot),
        rows: &[],
        mask: slot.mask,
        shift: slot.shift,
        addressing: slot.addressing,
        width: slot.width,
    };
    for found in runs(table) {
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

/// Ends the index `begin` started, as the index of `site` for the file `stamp` names; with `order`, each key's rows are
/// sorted by that column. An index over no row takes no room. A key of one cell whose cells lie close together is placed
/// by counting; any other is entered in the open table.
pub(crate) fn finish(
    arena: &mut IndexArena,
    site: IndexSite,
    stamp: FileStamp,
    order: Option<ColumnView<'_>>,
) {
    let rows = arena.pending.0;
    if rows == 0 {
        let slot = IndexSlot {
            stamp,
            table: Count(arena.entries.len()),
            words: Count(0),
            mask: Count(0),
            shift: HashShift(0),
            addressing: Addressing::Hashed,
            width: arena.width,
            first: Count(arena.rows.len()),
            count: Count(0),
        };
        kept(arena, site, slot);
        return;
    }
    let span = Count(
        usize::try_from(arena.bounds.greatest.0.saturating_sub(arena.bounds.least.0))
            .unwrap_or(usize::MAX),
    );
    let slot = if arena.width.0 == 1 && cells_lie(Count(rows), span) == KeyCells::Close {
        let least = arena.bounds.least;
        let slot = counted_slot(arena, stamp, least, span, Count(rows));
        let IndexArena {
            entries,
            rows: stretches,
            keys,
            base,
            ..
        } = arena;
        let offsets = || {
            keys.iter().map(|key| {
                Count(usize::try_from(key.0.wrapping_sub(least.0)).unwrap_or(usize::MAX))
            })
        };
        counted(
            table_of(entries, slot),
            stretches.get_mut(slot.first.0..).unwrap_or_default(),
            *base,
            offsets(),
            offsets(),
        );
        slot
    } else {
        let entries = rows
            .saturating_add(rows / 2)
            .max(LEAST_ENTRIES.0)
            .next_power_of_two();
        let stride = arena.width.0.saturating_add(1);
        let slot = IndexSlot {
            stamp,
            table: Count(arena.entries.len()),
            words: Count(entries.saturating_mul(stride)),
            mask: Count(entries.saturating_sub(1)),
            shift: HashShift(u64::BITS.saturating_sub(entries.trailing_zeros())),
            addressing: Addressing::Hashed,
            width: arena.width,
            first: Count(arena.rows.len()),
            count: Count(rows),
        };
        arena
            .entries
            .resize(slot.table.0.saturating_add(slot.words.0), EMPTY_RUN);
        arena
            .rows
            .resize(slot.first.0.saturating_add(rows), SourceRow(0));
        grouped(arena, slot);
        slot
    };
    if let Some(column) = order {
        sorted(arena, slot, column);
    }
    kept(arena, site, slot);
}
