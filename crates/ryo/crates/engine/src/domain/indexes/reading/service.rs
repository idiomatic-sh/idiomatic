//! The services of the `reading` part: a built index lent for reading, the rows of a key, every key's rows, and the part of
//! an ordered key's rows between two bounds.

use crate::domain::code::types::IndexSite;
use crate::domain::indexes::constants::{
    EMPTY_RUN, HASH_FOLD, HASH_MULTIPLIER, HASH_TURN, RUN_END, RUN_START_SHIFT,
};
use crate::domain::indexes::types::{
    Addressing, CellOfKey, ColumnView, HashShift, IndexArena, IndexRead, KeyMask, RangeEnd,
    RowHash, RowRun, SourceRow, TableWord,
};
use crate::domain::steps::types::BoundKind;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The cell of `row` in a column; the zero cell past the column's end.
#[inline]
pub(crate) fn cell_at(view: ColumnView<'_>, row: Count) -> CellWord {
    match view
        .cells
        .get(view.first.0.wrapping_add(row.0.wrapping_mul(view.stride.0)))
    {
        Some(cell) => *cell,
        None => CellWord(0),
    }
}

/// A hash with one more cell of a key mixed in.
#[inline]
pub(crate) const fn mixed(hash: RowHash, cell: CellWord) -> RowHash {
    RowHash((hash.0.rotate_left(HASH_TURN.0) ^ cell.0).wrapping_mul(HASH_MULTIPLIER.0))
}

/// The hash of a key, or of a row.
#[inline]
pub(crate) fn hashed(key: &[CellWord]) -> RowHash {
    key.iter().fold(RowHash(0), |hash, cell| mixed(hash, *cell))
}

/// A hash for a map that reads its low bits: the high half folded onto the low.
#[inline]
pub(crate) const fn folded(hash: RowHash) -> RowHash {
    RowHash(hash.0 ^ (hash.0 >> HASH_FOLD.0))
}

/// Where a key's walk of a hashed table starts.
#[inline]
pub(crate) fn start_of(shift: HashShift, key: impl Iterator<Item = CellWord>) -> Count {
    let hash = key.fold(RowHash(0), mixed);
    Count(usize::try_from(hash.0.checked_shr(shift.0).unwrap_or(0)).unwrap_or(usize::MAX))
}

/// How far a key's one cell lies past the least cell of an index addressed by the cell; past every place for a key of no
/// cell.
#[inline]
fn offset_of(first: Option<CellWord>, least: CellWord) -> Count {
    match first {
        Some(cell) => Count(usize::try_from(cell.0.wrapping_sub(least.0)).unwrap_or(usize::MAX)),
        None => Count(usize::MAX),
    }
}

/// The run an entry's last word holds.
#[inline]
pub(crate) fn run_from(word: TableWord) -> RowRun {
    RowRun {
        start: Count(usize::try_from(word.0 >> RUN_START_SHIFT.0).unwrap_or(usize::MAX)),
        end: Count(usize::try_from(word.0 & RUN_END.0).unwrap_or(usize::MAX)),
    }
}

/// The built index of `site`, lent for reading; an index of no row for a site that holds none.
#[inline]
pub(crate) fn read(arena: &IndexArena, site: IndexSite) -> IndexRead<'_> {
    let Some(slot) = arena
        .slots
        .get(usize::try_from(site.0).unwrap_or(usize::MAX))
    else {
        return IndexRead {
            entries: &[],
            rows: &[],
            mask: Count(0),
            shift: HashShift(0),
            addressing: Addressing::Hashed,
            width: Count(0),
            skipped: KeyMask(0),
            fixed: CellWord(0),
        };
    };
    IndexRead {
        entries: arena
            .entries
            .get(slot.table.0..slot.table.0.saturating_add(slot.words.0))
            .unwrap_or_default(),
        rows: arena
            .rows
            .get(slot.first.0..slot.first.0.saturating_add(slot.count.0))
            .unwrap_or_default(),
        mask: slot.mask,
        shift: slot.shift,
        addressing: slot.addressing,
        width: slot.width,
        skipped: slot.skipped,
        fixed: slot.fixed,
    }
}

/// A word of a counted table as a place among the index's rows.
#[inline]
fn place_of(word: TableWord) -> Count {
    Count(usize::try_from(word.0).unwrap_or(usize::MAX))
}

/// What an index does with the `at`-th cell of a key as it is asked.
#[inline]
fn key_cell(mask: KeyMask, at: Count) -> CellOfKey {
    let fixed = match u32::try_from(at.0) {
        Ok(place) => mask.0.checked_shr(place).unwrap_or(0) & 1,
        Err(_) => 0,
    };
    if fixed == 0 {
        CellOfKey::Kept
    } else {
        CellOfKey::Fixed
    }
}

/// The rows of a key in an index, the key being the cells the index keeps.
#[inline]
fn found<K: Iterator<Item = CellWord> + Clone>(index: IndexRead<'_>, key: K) -> RowRun {
    match index.addressing {
        Addressing::Rows(least) => {
            let at = offset_of(key.clone().next(), least);
            if at.0 < index.rows.len() {
                RowRun {
                    start: at,
                    end: Count(at.0.wrapping_add(1)),
                }
            } else {
                RowRun::default()
            }
        }
        Addressing::Counted(least) => {
            let at = offset_of(key.clone().next(), least);
            match (
                index.entries.get(at.0),
                index.entries.get(at.0.wrapping_add(1)),
            ) {
                (Some(start), Some(end)) => RowRun {
                    start: place_of(*start),
                    end: place_of(*end),
                },
                (Some(_) | None, None) | (None, Some(_)) => RowRun::default(),
            }
        }
        Addressing::Hashed => {
            let stride = index.width.0.saturating_add(1);
            let mut at = start_of(index.shift, key.clone()).0;
            loop {
                let from = at.wrapping_mul(stride);
                let Some((run, held)) = index
                    .entries
                    .get(from..from.wrapping_add(stride))
                    .and_then(<[TableWord]>::split_last)
                else {
                    return RowRun::default();
                };
                if *run == EMPTY_RUN {
                    return RowRun::default();
                }
                if held
                    .iter()
                    .zip(key.clone())
                    .all(|(cell, asked)| cell.0 == asked.0)
                {
                    return run_from(*run);
                }
                at = at.wrapping_add(1) & index.mask.0;
            }
        }
    }
}

/// The rows of `key` in an index: the stretch of the index's rows that hold it, empty when none does. An index of no row
/// answers after one test, whatever its key. Where the index keeps no cell of some of the key's (every row of its source
/// holds one fixed cell there), the key has rows only if it asks that cell there, and is found by its other cells.
#[inline]
pub(crate) fn run_of(index: IndexRead<'_>, key: &[CellWord]) -> RowRun {
    if index.rows.is_empty() {
        return RowRun::default();
    }
    if index.skipped.0 == 0 {
        return found(index, key.iter().copied());
    }
    let placed = || key.iter().copied().enumerate();
    if placed().any(|(at, cell)| {
        key_cell(index.skipped, Count(at)) == CellOfKey::Fixed && cell != index.fixed
    }) {
        return RowRun::default();
    }
    found(
        index,
        placed()
            .filter(|(at, _)| key_cell(index.skipped, Count(*at)) == CellOfKey::Kept)
            .map(|(_, cell)| cell),
    )
}

/// The rows of a run, as their numbers in the source.
#[inline]
pub(crate) fn rows_of<'a>(index: IndexRead<'a>, run: RowRun) -> &'a [SourceRow] {
    index.rows.get(run.start.0..run.end.0).unwrap_or_default()
}

/// A key's run of a site's index as a stretch of the arena's rows: the run moved by where the index's rows start. A
/// stretch stays good while the file lasts, whatever is built after it.
#[must_use]
pub(crate) fn stretch_of(arena: &IndexArena, site: IndexSite, run: RowRun) -> RowRun {
    let first = match arena
        .slots
        .get(usize::try_from(site.0).unwrap_or(usize::MAX))
    {
        Some(slot) => slot.first.0,
        None => 0,
    };
    RowRun {
        start: Count(first.saturating_add(run.start.0)),
        end: Count(first.saturating_add(run.end.0)),
    }
}

/// Every key of an index that holds a row, with its run, in the table's order: what a fill that groups its input walks.
pub(crate) fn runs(index: IndexRead<'_>) -> impl Iterator<Item = RowRun> + '_ {
    let none: &[TableWord] = &[];
    let (hashed, counted, own) = match index.addressing {
        Addressing::Hashed => (index.entries, none, 0),
        Addressing::Counted(_) => (none, index.entries, 0),
        Addressing::Rows(_) => (none, none, index.rows.len()),
    };
    hashed
        .chunks_exact(index.width.0.saturating_add(1))
        .filter_map(|entry| entry.last().copied())
        .filter(|run| *run != EMPTY_RUN)
        .map(run_from)
        .chain(counted.windows(2).filter_map(|pair| match pair {
            [start, end] if start != end => Some(RowRun {
                start: place_of(*start),
                end: place_of(*end),
            }),
            _ => None,
        }))
        .chain((0..own).map(|at| RowRun {
            start: Count(at),
            end: Count(at.wrapping_add(1)),
        }))
}

/// The part of an ordered run whose order cells lie between two bounds. An absent bound leaves that side open.
pub(crate) fn run_between(
    index: IndexRead<'_>,
    run: RowRun,
    order: ColumnView<'_>,
    low: Option<RangeEnd>,
    high: Option<RangeEnd>,
) -> RowRun {
    let rows = rows_of(index, run);
    let cell =
        |row: &SourceRow| cell_at(order, Count(usize::try_from(row.0).unwrap_or(usize::MAX)));
    let start = low.map_or(0, |bound| {
        rows.partition_point(|row| match bound.end {
            BoundKind::Inclusive => cell(row) < bound.cell,
            BoundKind::Strict => cell(row) <= bound.cell,
        })
    });
    let end = high.map_or(rows.len(), |bound| {
        rows.partition_point(|row| match bound.end {
            BoundKind::Inclusive => cell(row) <= bound.cell,
            BoundKind::Strict => cell(row) < bound.cell,
        })
    });
    RowRun {
        start: Count(run.start.0.saturating_add(start)),
        end: Count(run.start.0.saturating_add(end.max(start))),
    }
}
