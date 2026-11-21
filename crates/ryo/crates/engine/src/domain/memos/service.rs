//! The services of the `memos` concept: an answer asked for and kept by the site that asks and every cell of its key. One
//! map for every site; nothing is allocated and no clock is read when an answer is kept.

use super::constants::{HASH_FOLD, SITE_MULTIPLIER};
use super::types::{CellSpan, EntryCharge, MemoEntry, MemoId, MemoMap};
use crate::domain::code::types::MemoSite;
use crate::domain::indexes::reading::service::hashed;
use crate::domain::indexes::types::RowHash;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Forgets every answer of the last file, keeping the room.
pub(crate) fn clear_memos(map: &mut MemoMap) {
    map.entries.clear();
    map.cells.clear();
    map.found.clear();
    map.charges.clear();
    map.reached.clear();
}

/// The cells of a span.
#[inline]
fn span(cells: &[CellWord], at: CellSpan) -> &[CellWord] {
    cells
        .get(at.start.0..at.start.0.saturating_add(at.length.0))
        .unwrap_or_default()
}

/// The cells an answer is kept by: every cell of its key. The one place that says so: the hash and the comparison both
/// read it.
#[inline]
fn kept_by(key: &[CellWord]) -> &[CellWord] {
    key
}

/// The hash an answer is kept by: its site and every cell of its key.
#[inline]
fn memo_hash(site: MemoSite, key: &[CellWord]) -> RowHash {
    let hash = hashed(kept_by(key)).0 ^ u64::from(site.0).wrapping_mul(SITE_MULTIPLIER.0);
    RowHash(hash ^ (hash >> HASH_FOLD.0))
}

/// The entry of a kept answer.
#[inline]
fn entry(entries: &[MemoEntry], answer: MemoId) -> Option<&MemoEntry> {
    entries.get(usize::try_from(answer.0).unwrap_or(usize::MAX))
}

/// The answer kept for `site` and `key`, when one is.
#[inline]
pub(crate) fn asked(map: &MemoMap, site: MemoSite, key: &[CellWord]) -> Option<MemoId> {
    map.found
        .find(memo_hash(site, key).0, |answer| {
            entry(&map.entries, *answer).is_some_and(|kept| {
                kept.site == site && kept_by(span(&map.cells, kept.key)) == kept_by(key)
            })
        })
        .copied()
}

/// Keeps for `site` and `key` the answer `rows`, which is `count` rows, row by row.
pub(crate) fn keep(
    map: &mut MemoMap,
    site: MemoSite,
    key: &[CellWord],
    rows: &[CellWord],
    count: Count,
) -> MemoId {
    let MemoMap {
        entries,
        cells,
        found,
        ..
    } = map;
    let key_at = CellSpan {
        start: Count(cells.len()),
        length: Count(key.len()),
    };
    cells.extend_from_slice(key);
    let rows_at = CellSpan {
        start: Count(cells.len()),
        length: Count(rows.len()),
    };
    cells.extend_from_slice(rows);
    let answer = MemoId(u32::try_from(entries.len()).unwrap_or(u32::MAX));
    entries.push(MemoEntry {
        site,
        key: key_at,
        rows: rows_at,
        count,
    });
    drop(
        found.insert_unique(memo_hash(site, key).0, answer, |other| {
            entry(entries, *other).map_or(0, |kept| memo_hash(kept.site, span(cells, kept.key)).0)
        }),
    );
    answer
}

/// How many rows a kept answer has.
#[inline]
pub(crate) fn row_count(map: &MemoMap, answer: MemoId) -> Count {
    entry(&map.entries, answer).map_or(Count(0), |kept| kept.count)
}

/// The cells of a kept answer's rows, row by row.
#[inline]
pub(crate) fn rows(map: &MemoMap, answer: MemoId) -> &[CellWord] {
    entry(&map.entries, answer).map_or(&[], |kept| span(&map.cells, kept.rows))
}

/// Records what an answer of a pattern cost and reached: its own work, its unit, and the answers it asked directly.
pub(crate) fn set_charge(
    map: &mut MemoMap,
    answer: MemoId,
    mut charge: EntryCharge,
    asked: &[MemoId],
) {
    charge.reached = CellSpan {
        start: Count(map.reached.len()),
        length: Count(asked.len()),
    };
    map.reached.extend_from_slice(asked);
    let place = usize::try_from(answer.0).unwrap_or(usize::MAX);
    if map.charges.len() <= place {
        map.charges
            .resize(place.saturating_add(1), EntryCharge::default());
    }
    if let Some(slot) = map.charges.get_mut(place) {
        *slot = charge;
    }
}

/// What an answer cost and reached, to be charged; none for an answer that is no pattern's.
#[inline]
pub(crate) fn charge_mut(map: &mut MemoMap, answer: MemoId) -> Option<&mut EntryCharge> {
    map.charges
        .get_mut(usize::try_from(answer.0).unwrap_or(usize::MAX))
}

/// The answers a stretch of the map's lists names.
#[inline]
pub(crate) fn reached_at(map: &MemoMap, at: CellSpan) -> &[MemoId] {
    map.reached
        .get(at.start.0..at.start.0.saturating_add(at.length.0))
        .unwrap_or_default()
}
