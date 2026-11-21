//! The services of the `keyed` part: a relation that is not whole asked of the language for one key, through a site. The
//! key is asked once per file: its rows are kept as a stretch of the store's rows, found again by the site and the key's
//! cells. A site of several columns asks one of them and keeps the rows that hold the other cells.

use crate::domain::code::service::list;
use crate::domain::code::types::{IndexSite, RelationNumber};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::service::{asked_among, keyed};
use crate::domain::filled::types::{FilledState, MissingCode, RelationAsk};
use crate::domain::indexes::reading::service::{folded, mixed};
use crate::domain::indexes::types::{RowHash, RowRun, SourceRow};
use crate::domain::made_tables::reading::service::{relation_at, site_at};
use crate::domain::made_tables::types::{AskedStretch, FillFrame, KeyAnswer, KeyRows};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The hash a key asked through a site is kept by: the site, then every cell.
#[inline]
fn asked_hash(site: IndexSite, key: &[CellWord]) -> RowHash {
    folded(key.iter().fold(
        mixed(RowHash(0), CellWord(u64::from(site.0))),
        |hash, cell| mixed(hash, *cell),
    ))
}

/// The cells of a key kept.
#[inline]
fn kept_cells<'c>(cells: &'c [CellWord], asked: &AskedStretch) -> &'c [CellWord] {
    cells
        .get(asked.cells.0..asked.cells.0.saturating_add(asked.width.0))
        .unwrap_or_default()
}

/// The rows of the relation whose columns, the site's, hold `key`, while the relation is not whole: the language is asked
/// for the key alone the first time, and the rows are found again after. When the relation is whole, or the language
/// answers every row and so makes it whole, the answer says so and the site's index is read instead.
///
/// # Errors
///
/// The language refused the relation or wrote it in part; or the site's columns are not the key's.
pub(crate) fn key_asked(
    frame: &mut FillFrame<'_>,
    site: IndexSite,
    number: RelationNumber,
    key: &[CellWord],
) -> Result<KeyAnswer, FilledError> {
    let programs = frame.programs;
    let stamp = frame.store.stamp;
    let whole = frame
        .store
        .relations
        .0
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
        .is_some_and(|slot| slot.stamp == stamp && slot.state == FilledState::Whole);
    if whole {
        return Ok(KeyAnswer::Whole(Count(0)));
    }
    let hash = asked_hash(site, key);
    let cells = &frame.store.asked_cells;
    if let Some(held) = frame.store.asked.find(hash.0, |each| {
        each.site == site && kept_cells(cells, each) == key
    }) {
        return Ok(KeyAnswer::Rows(KeyRows {
            rows: held.rows,
            ..KeyRows::default()
        }));
    }
    let places = list(&programs.code.pool, site_at(&programs.code, site)?.columns);
    let relation = relation_at(&programs.code, number)?;
    let chosen = asked_among(&mut frame.store.relations, number, relation, places).0;
    let wrong = FilledError::Code {
        named: MissingCode::Site,
    };
    let (Some(place), Some(cell)) = (places.get(chosen), key.get(chosen)) else {
        return Err(wrong);
    };
    if places.len() != key.len() {
        return Err(wrong);
    }
    let answer = keyed(
        &mut frame.store.relations,
        RelationAsk {
            number,
            relation,
            stamp,
            facts: &mut *frame.facts,
            words: &mut *frame.words,
        },
        Count(usize::try_from(place.0).unwrap_or(usize::MAX)),
        *cell,
    )?;
    if answer.state == FilledState::Whole {
        return Ok(KeyAnswer::Whole(answer.filled));
    }
    // The rows the answer wrote that hold the key's other cells, as a stretch of the store's rows.
    let columns = frame
        .store
        .relations
        .0
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
        .map_or(&[][..], |slot| slot.columns.0.as_slice());
    let start = frame.store.indexes.rows.len();
    for row in answer.rows.start.0..answer.rows.end.0 {
        let holds = places.iter().zip(key).all(|(place, cell)| {
            columns
                .get(usize::try_from(place.0).unwrap_or(usize::MAX))
                .and_then(|column| column.0.get(row))
                == Some(cell)
        });
        if holds {
            frame
                .store
                .indexes
                .rows
                .push(SourceRow(u32::try_from(row).unwrap_or(u32::MAX)));
        }
    }
    let rows = RowRun {
        start: Count(start),
        end: Count(frame.store.indexes.rows.len()),
    };
    let kept = AskedStretch {
        site,
        cells: Count(frame.store.asked_cells.len()),
        width: Count(key.len()),
        rows,
    };
    frame.store.asked_cells.extend_from_slice(key);
    let cells = &frame.store.asked_cells;
    drop(frame.store.asked.insert_unique(hash.0, kept, |each| {
        asked_hash(each.site, kept_cells(cells, each)).0
    }));
    Ok(KeyAnswer::Rows(KeyRows {
        rows,
        filled: answer.filled,
        ..KeyRows::default()
    }))
}
