//! The services of the `filled` concept: a relation asked of the language whole or for one key, each once per file. A fill
//! appends: the rows a key's answer wrote stay where they are, and a whole fill's rows start after them.

use super::errors::FilledError;
use super::types::{
    AskedKey, AskedRows, FilledRelations, FilledState, KeptFault, MissingCode, RelationAsk,
    RelationSlot,
};
use crate::domain::code::types::{RelationNumber, WordSlot};
use crate::domain::indexes::reading::service::{folded, mixed};
use crate::domain::indexes::types::{FileStamp, RowHash, RowRun};
use ryo_language::domain::facts::types::{CellWord, FactAnswer, FillExtent, Wanted};
use ryo_relations::domain::catalog::service::key_columns;
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{Count, Relation};

/// The hash the keys asked are kept by: the column's place and the cell.
#[inline]
fn key_hash(column: Count, cell: CellWord) -> RowHash {
    let place = CellWord(u64::try_from(column.0).unwrap_or(u64::MAX));
    folded(mixed(mixed(RowHash(0), place), cell))
}

/// The relation's slot for the file `stamp` names: emptied, its buffers kept, when it holds another file's rows.
fn slot_of(
    relations: &mut FilledRelations,
    number: RelationNumber,
    relation: Relation,
    stamp: FileStamp,
) -> Result<&mut RelationSlot, FilledError> {
    let place = usize::try_from(number.0).unwrap_or(usize::MAX);
    if relations.0.len() <= place {
        relations
            .0
            .resize_with(place.saturating_add(1), RelationSlot::default);
    }
    let slot = relations.0.get_mut(place).ok_or(FilledError::Code {
        named: MissingCode::Relation,
    })?;
    if slot.stamp != stamp {
        slot.stamp = stamp;
        slot.state = FilledState::Unfilled;
        for column in &mut slot.columns.0 {
            column.0.clear();
        }
        slot.rows = Count(0);
        slot.whole = Count(0);
        slot.asked.clear();
        slot.fault = None;
    }
    if slot.named.is_empty() {
        slot.named = columns(relation);
    }
    Ok(slot)
}

/// Which of a site's columns a key is asked of the language by: the first the relation's catalog item declares a key, for
/// which the language has a body; else the first. Answers its place among `places`, the columns' places in the relation.
pub(crate) fn asked_among(
    relations: &mut FilledRelations,
    number: RelationNumber,
    relation: Relation,
    places: &[WordSlot],
) -> Count {
    let declared = key_columns(relation);
    let at = usize::try_from(number.0).unwrap_or(usize::MAX);
    if relations.0.len() <= at {
        relations
            .0
            .resize_with(at.saturating_add(1), RelationSlot::default);
    }
    let named = match relations
        .0
        .get_mut(usize::try_from(number.0).unwrap_or(usize::MAX))
    {
        Some(slot) => {
            if slot.named.is_empty() {
                slot.named = columns(relation);
            }
            slot.named.as_slice()
        }
        None => &[],
    };
    Count(
        places
            .iter()
            .position(|place| {
                named
                    .get(usize::try_from(place.0).unwrap_or(usize::MAX))
                    .is_some_and(|column| declared.contains(column))
            })
            .unwrap_or(0),
    )
}

/// The rows of the relation's whole fill for the file `stamp` names, from where it starts: no rows while the relation is
/// not whole. The one read of "is this relation whole for the file".
pub(crate) fn whole_rows(
    relations: &FilledRelations,
    number: RelationNumber,
    stamp: FileStamp,
) -> RowRun {
    match relations
        .0
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
    {
        Some(slot) if slot.stamp == stamp && slot.state == FilledState::Whole => RowRun {
            start: slot.whole,
            end: slot.rows,
        },
        Some(_) | None => RowRun::default(),
    }
}

/// Whether the relation is whole for the file `stamp` names.
pub(crate) fn held_state(
    relations: &FilledRelations,
    number: RelationNumber,
    stamp: FileStamp,
) -> FilledState {
    match relations
        .0
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
    {
        Some(slot) if slot.stamp == stamp => slot.state,
        Some(_) | None => FilledState::Unfilled,
    }
}

/// How many rows the relation holds for the file `stamp` names: the rows every ask so far appended.
pub(crate) fn rows_held(
    relations: &FilledRelations,
    number: RelationNumber,
    stamp: FileStamp,
) -> Count {
    match relations
        .0
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
    {
        Some(slot) if slot.stamp == stamp => slot.rows,
        Some(_) | None => Count(0),
    }
}

/// The fault of a relation the language did not write, as every read of it answers.
fn faulted(relation: Relation, fault: KeptFault) -> FilledError {
    match fault {
        KeptFault::Refused(cause) => FilledError::Refused { relation, cause },
        KeptFault::Partial(rows) => FilledError::Partial { relation, rows },
    }
}

/// Takes the language's answer to one ask: the rows are those past `before`, or the fault is kept for every later read.
fn answered(
    slot: &mut RelationSlot,
    relation: Relation,
    answer: FactAnswer,
) -> Result<FillExtent, FilledError> {
    let fault = match answer {
        FactAnswer::Answered(extent) => {
            slot.rows = Count(slot.columns.0.first().map_or(0, |column| column.0.len()));
            return Ok(extent);
        }
        FactAnswer::Partial(rows) => KeptFault::Partial(rows),
        FactAnswer::Refused(cause) => KeptFault::Refused(cause),
    };
    slot.fault = Some(fault.clone());
    Err(faulted(relation, fault))
}

/// Every row of the relation: asked whole of the language the first time, its rows written after those the keys asked
/// before appended; the rows of the whole fill after that.
///
/// # Errors
///
/// The language refused the relation or wrote it in part: asked once, answered to every read of the file.
pub(crate) fn whole(
    relations: &mut FilledRelations,
    ask: RelationAsk<'_>,
) -> Result<AskedRows, FilledError> {
    let slot = slot_of(relations, ask.number, ask.relation, ask.stamp)?;
    if let Some(fault) = &slot.fault {
        return Err(faulted(ask.relation, fault.clone()));
    }
    let before = slot.rows;
    let filled = match slot.state {
        FilledState::Whole => Count(0),
        FilledState::Unfilled | FilledState::Keyed => {
            let answer = ask
                .facts
                .fill(ask.relation, Wanted::Whole, ask.words, &mut slot.columns);
            let _extent = answered(slot, ask.relation, answer)?;
            slot.whole = before;
            slot.state = FilledState::Whole;
            Count(slot.rows.0.saturating_sub(before.0))
        }
    };
    Ok(AskedRows {
        rows: RowRun {
            start: slot.whole,
            end: slot.rows,
        },
        filled,
        state: FilledState::Whole,
    })
}

/// The rows the language wrote for one key: the cell of the column at `column`'s place. Asked of the language the first
/// time the key is asked while the relation is not whole. A language that answers every row makes the relation whole: the
/// answer says so, and the rows are then every row; so they are when the relation was whole before the ask.
///
/// # Errors
///
/// The language refused the relation or wrote it in part; or the relation has no column at that place.
pub(crate) fn keyed(
    relations: &mut FilledRelations,
    ask: RelationAsk<'_>,
    column: Count,
    cell: CellWord,
) -> Result<AskedRows, FilledError> {
    let slot = slot_of(relations, ask.number, ask.relation, ask.stamp)?;
    if let Some(fault) = &slot.fault {
        return Err(faulted(ask.relation, fault.clone()));
    }
    let whole = |slot: &RelationSlot, filled: Count| AskedRows {
        rows: RowRun {
            start: slot.whole,
            end: slot.rows,
        },
        filled,
        state: FilledState::Whole,
    };
    if slot.state == FilledState::Whole {
        return Ok(whole(slot, Count(0)));
    }
    let hash = key_hash(column, cell);
    if let Some(held) = slot
        .asked
        .find(hash.0, |asked| asked.column == column && asked.cell == cell)
    {
        return Ok(AskedRows {
            rows: held.rows,
            filled: Count(0),
            state: FilledState::Keyed,
        });
    }
    let named = *slot.named.get(column.0).ok_or(FilledError::Code {
        named: MissingCode::Column,
    })?;
    let before = slot.rows;
    let answer = ask.facts.fill(
        ask.relation,
        Wanted::Key(named, cell),
        ask.words,
        &mut slot.columns,
    );
    let extent = answered(slot, ask.relation, answer)?;
    let filled = Count(slot.rows.0.saturating_sub(before.0));
    match extent {
        FillExtent::Whole => {
            slot.whole = before;
            slot.state = FilledState::Whole;
            Ok(whole(slot, filled))
        }
        FillExtent::Asked => {
            let rows = RowRun {
                start: before,
                end: slot.rows,
            };
            slot.state = FilledState::Keyed;
            drop(
                slot.asked
                    .insert_unique(hash.0, AskedKey { column, cell, rows }, |asked| {
                        key_hash(asked.column, asked.cell).0
                    }),
            );
            Ok(AskedRows {
                rows,
                filled,
                state: FilledState::Keyed,
            })
        }
    }
}
