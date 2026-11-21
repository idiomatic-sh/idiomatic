//! The services of the `counting` part: an index of one cell that lies close above a least cell, its rows placed by
//! counting into a table of places; and the index of a source's rows by their own id, which is built by writing nothing.

use crate::domain::code::types::IndexSite;
use crate::domain::indexes::constants::{COUNTED_WORDS, LEAST_ENTRIES, PLACED_SPAN};
use crate::domain::indexes::slots::service::{kept, table_of};
use crate::domain::indexes::types::{
    Addressing, CountedKey, FileStamp, HashShift, IndexArena, IndexSlot, KeyCells, SourceRow,
    TableWord,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The build by counting, of a key that is one cell less the least: `offsets` gives, row after row and twice over, how
/// far each row's cell lies past the least. The first pass counts each cell's rows, a pass over the table turns the counts
/// into where each cell's stretch starts, and the second pass writes each row at its cell's next place. No key is copied,
/// hashed or compared, and the rows of a stretch are ascending. The table is zero before: word `c + 2` counts the cell
/// `c`, so that after the second pass word `c` is where its stretch starts and word `c + 1` where it ends.
#[inline]
pub(crate) fn counted<K: Iterator<Item = Count>>(
    table: &mut [TableWord],
    stretches: &mut [SourceRow],
    base: SourceRow,
    first: K,
    second: K,
) {
    // Rows that follow one another with the same cell are counted in hand, and written to the table when the cell
    // changes: a count read back from the table for every row would wait on the write before it.
    let (mut last, mut run) = (Count(usize::MAX), 0_u64);
    for offset in first {
        if offset == last {
            run = run.wrapping_add(1);
        } else {
            if run != 0
                && let Some(count) = table.get_mut(last.0.wrapping_add(2))
            {
                count.0 = count.0.wrapping_add(run);
            }
            last = offset;
            run = 1;
        }
    }
    if run != 0
        && let Some(count) = table.get_mut(last.0.wrapping_add(2))
    {
        count.0 = count.0.wrapping_add(run);
    }
    // Each word becomes the rows counted up to it: word `c + 2` where the stretch of `c` ends, word `c + 1` where it
    // starts.
    let mut next = 0_u64;
    for word in table.iter_mut() {
        next = next.wrapping_add(word.0);
        word.0 = next;
    }
    // The same in the second pass: a cell's next place is held in hand while the cell stays, and kept in the table
    // when it changes. A cell with no word of the table has no place, and its rows are passed.
    let no_place = u64::MAX;
    let (mut last, mut cursor) = (Count(usize::MAX), no_place);
    let mut row = base.0;
    for offset in second {
        if offset != last {
            if cursor != no_place
                && let Some(held) = table.get_mut(last.0.wrapping_add(1))
            {
                held.0 = cursor;
            }
            last = offset;
            cursor = table
                .get(last.0.wrapping_add(1))
                .map_or(no_place, |held| held.0);
        }
        if cursor != no_place {
            if let Some(cell) = stretches.get_mut(usize::try_from(cursor).unwrap_or(usize::MAX)) {
                *cell = SourceRow(row);
            }
            cursor = cursor.wrapping_add(1);
        }
        row = row.wrapping_add(1);
    }
    if cursor != no_place
        && let Some(held) = table.get_mut(last.0.wrapping_add(1))
    {
        held.0 = cursor;
    }
}

/// An index of `rows` rows whose table is to be counted: its slot, with the table zeroed and the rows there. `span` is
/// how far the greatest cell lies past the least.
pub(crate) fn counted_slot(
    arena: &mut IndexArena,
    stamp: FileStamp,
    least: CellWord,
    span: Count,
    rows: Count,
) -> IndexSlot {
    let slot = IndexSlot {
        stamp,
        table: Count(arena.entries.len()),
        words: Count(span.0.saturating_add(COUNTED_WORDS.0)),
        mask: Count(0),
        shift: HashShift(0),
        addressing: Addressing::Counted(least),
        width: Count(1),
        first: Count(arena.rows.len()),
        count: rows,
    };
    arena
        .entries
        .resize(slot.table.0.saturating_add(slot.words.0), TableWord(0));
    arena
        .rows
        .resize(slot.first.0.saturating_add(rows.0), SourceRow(0));
    slot
}

/// How the cells of a one-cell key over `rows` rows lie, when they span `span`: close enough together that a table of a
/// place for each is no larger than the rows it places a few times over, or apart.
#[inline]
pub(crate) const fn cells_lie(rows: Count, span: Count) -> KeyCells {
    if rows.0 > 0
        && span.0
            < rows
                .0
                .saturating_mul(PLACED_SPAN.0)
                .saturating_add(LEAST_ENTRIES.0)
    {
        KeyCells::Close
    } else {
        KeyCells::Apart
    }
}

/// Builds, as the index of `site` for the file `stamp` names, the index of a source's `rows` rows by one cell that lies
/// at most `span` past `least`, by counting: `offsets` gives, each time it is asked, every row's cell less the least, row
/// after row. The source's first row is numbered nought. For a key the source already holds as a column of small numbers:
/// two passes over that column and nothing else.
pub(crate) fn place<K: Iterator<Item = Count>>(
    arena: &mut IndexArena,
    site: IndexSite,
    stamp: FileStamp,
    counted_by: CountedKey,
    offsets: impl Fn() -> K,
) {
    let slot = counted_slot(
        arena,
        stamp,
        counted_by.least,
        counted_by.span,
        counted_by.rows,
    );
    let IndexArena { entries, rows, .. } = arena;
    counted(
        table_of(entries, slot),
        rows.get_mut(slot.first.0..).unwrap_or_default(),
        SourceRow(0),
        offsets(),
        offsets(),
    );
    kept(arena, site, slot);
}

/// Builds, as the index of `site` for the file `stamp` names, the index of a source's `rows` rows by their own id, the
/// id of row `r` being `least + r`: nothing is written when the arena's front holds that many rows that number
/// themselves, which it does from the second file that asks.
pub(crate) fn own_rows(
    arena: &mut IndexArena,
    site: IndexSite,
    stamp: FileStamp,
    least: CellWord,
    rows: Count,
) {
    let first = if rows.0 <= arena.own.0 {
        Count(0)
    } else {
        // The front is too short for this file: this index holds its own rows, and the front is grown when the next
        // file starts.
        arena.wanted = Count(arena.wanted.0.max(rows.0));
        let first = arena.rows.len();
        arena
            .rows
            .extend((0..u32::try_from(rows.0).unwrap_or(u32::MAX)).map(SourceRow));
        Count(first)
    };
    let slot = IndexSlot {
        stamp,
        table: Count(arena.entries.len()),
        words: Count(0),
        mask: Count(0),
        shift: HashShift(0),
        addressing: Addressing::Rows(least),
        width: Count(1),
        first,
        count: rows,
    };
    kept(arena, site, slot);
}
