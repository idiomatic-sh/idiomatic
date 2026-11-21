//! The services of the `making` part: a table made for the file, on its first read and once. What its fill reads is made,
//! filled or indexed first, in the order its stages read it; then its pipelines run. A pipeline whose scan has no row
//! runs nothing and has nothing made for it.

use crate::domain::code::filling::service::{stage_scan_of, stage_write_of};
use crate::domain::code::types::{CodeEntry, RelationNumber};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::service::whole;
use crate::domain::filled::types::{MissingCode, RelationAsk, UnbuiltFill};
use crate::domain::indexes::service::state_of;
use crate::domain::indexes::types::IndexBuilt;
use crate::domain::made_tables::carrying::service::stage_reach;
use crate::domain::made_tables::fanning::service::scan_read;
use crate::domain::made_tables::fills::service::{constant_rows, fill_at, grouped_rows, input_at};
use crate::domain::made_tables::fills::types::{FillInputs, FillKind, TableFill};
use crate::domain::made_tables::indexing::service::index_built;
use crate::domain::made_tables::matching::service::match_rows;
use crate::domain::made_tables::matching::types::MatchSide;
use crate::domain::made_tables::pipelines::service::piped;
use crate::domain::made_tables::reading::service::{
    pipeline_words, relation_at, site_at, site_source,
};
use crate::domain::made_tables::service::clear_rows;
use crate::domain::made_tables::staging::service::WRONG_WORD;
use crate::domain::made_tables::staging::types::BatchFrame;
use crate::domain::made_tables::types::PipeRun;
use crate::domain::made_tables::types::{
    FillFrame, FillKept, FillWork, MadeNow, MadeState, PatternRows, Reached, ReadSource,
    SourceWork, TableNeed, TableRows, TableSlot,
};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::relations::types::Count;

/// A table's slot, there for any table the code numbers.
fn slot_mut<'s>(frame: &'s mut FillFrame<'_>, table: MadeTableId) -> Option<&'s mut TableSlot> {
    let place = usize::try_from(table.0).unwrap_or(usize::MAX);
    if place >= frame.programs.code.tables.len() {
        return None;
    }
    if frame.store.tables.len() <= place {
        frame
            .store
            .tables
            .resize_with(place.saturating_add(1), TableSlot::default);
    }
    frame.store.tables.get_mut(place)
}

/// Fills a relation whole for the file if it is not whole. Answers the rows the language wrote for it now.
fn filled_whole(frame: &mut FillFrame<'_>, number: RelationNumber) -> Result<Count, FilledError> {
    let relation = relation_at(frame.programs.code, number)?;
    let ask = RelationAsk {
        number,
        relation,
        stamp: frame.store.stamp,
        facts: &mut *frame.facts,
        words: &mut *frame.words,
    };
    whole(&mut frame.store.relations, ask).map(|rows| rows.filled)
}

/// Makes a table the fill side reaches itself, unless the file has it: the loop is asked whether it may have a row; a
/// table that has none is kept as made with no row, and no fill runs.
fn reached_table(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
) -> Result<(), FilledError> {
    let stamp = frame.store.stamp;
    let slot = slot_mut(frame, table).ok_or(FilledError::Code {
        named: MissingCode::Table,
    })?;
    if slot.stamp == stamp && slot.state == MadeState::Made {
        return Ok(());
    }
    match uses.table_need(frame, table)? {
        TableNeed::Rows => made_now(frame, uses, table).map(|_| ()),
        TableNeed::Empty => {
            let width = fill_at(frame.programs.code, table)?.width;
            let slot = slot_mut(frame, table).ok_or(FilledError::Code {
                named: MissingCode::Table,
            })?;
            // The decision may have made it: a need that asks the table itself.
            if slot.stamp == stamp && slot.state == MadeState::Made {
                return Ok(());
            }
            slot.stamp = stamp;
            slot.state = MadeState::Made;
            slot.work = FillWork::default();
            slot.reached.clear();
            clear_rows(&mut slot.rows, width);
            Ok(())
        }
    }
}

/// Has a thing the fill side reaches there for the file: a table made, a relation whole, an index built over its source.
/// Answers what that made the engine do now: nothing when it was there.
///
/// # Errors
///
/// The source could not be read, or the code does not hold it.
pub(crate) fn ensured(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    reached: Reached,
) -> Result<SourceWork, FilledError> {
    let mut work = SourceWork::default();
    match reached {
        Reached::Nodes => {}
        Reached::Relation(number) => work.filled = filled_whole(frame, number)?,
        Reached::Table(table) => reached_table(frame, uses, table)?,
        Reached::Index(site) => {
            if state_of(&frame.store.indexes, site, frame.store.stamp) == IndexBuilt::Built {
                return Ok(work);
            }
            let programs = frame.programs;
            match site_source(programs.code, site_at(programs.code, site)?)? {
                ReadSource::Nodes(_) => {}
                ReadSource::Filled(number) => work.filled = filled_whole(frame, number)?,
                ReadSource::Made(table) => reached_table(frame, uses, table)?,
                ReadSource::Written(_) => {
                    return Err(FilledError::Unbuilt {
                        unbuilt: UnbuiltFill::Written,
                    });
                }
            }
            work.indexed = index_built(frame, site)?;
        }
    }
    Ok(work)
}

/// Runs the pipelines of one input into `into`: for each, what its scan reads is there first; a scan with no row ends
/// the pipeline; else what its other stages read is there, in stage order, and the pipeline runs.
fn input_piped(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
    entries: FillInputs<'_>,
    into: &mut TableRows,
    kept: FillKept<'_>,
) -> Result<(), FilledError> {
    let FillKept { work, reached } = kept;
    let programs = frame.programs;
    for entry in entries.entries {
        let entry = CodeEntry(entry.0);
        let words = pipeline_words(programs.code, entry)?;
        let mut scanned = Count(0);
        for (at, word) in words.iter().enumerate() {
            if let Some(read) = stage_reach(programs.code, *word)? {
                let _work = ensured(frame, uses, read)?;
                reached.push(read);
            }
            if at == 0 {
                let batch = BatchFrame {
                    programs,
                    store: frame.store,
                    nodes: frame.nodes,
                    file: frame.file,
                    text: frame.text,
                };
                scanned = scan_read(batch, stage_scan_of(*word))?.count;
                if scanned.0 == 0 {
                    break;
                }
            }
        }
        if scanned.0 == 0 {
            continue;
        }
        let mut room = frame.store.rooms.pop().unwrap_or_default();
        let done = piped(
            frame,
            uses,
            entry,
            &mut PipeRun {
                table,
                into,
                work,
                room: &mut room,
            },
        );
        frame.store.rooms.push(room);
        done?;
    }
    Ok(())
}

/// How many cells a row of an input's pipelines has: what its first pipeline's write writes.
fn input_width(frame: &FillFrame<'_>, input: FillInputs<'_>) -> Result<Count, FilledError> {
    let Some(entry) = input.entries.first() else {
        return Ok(Count(0));
    };
    let words = pipeline_words(frame.programs.code, CodeEntry(entry.0))?;
    let write = words.last().ok_or(WRONG_WORD)?;
    Ok(Count(usize::from(stage_write_of(*write).columns.0)))
}

/// Runs a table's fill into `rows`.
fn filled_now(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
    fill: TableFill,
    rows: &mut TableRows,
    kept: FillKept<'_>,
) -> Result<(), FilledError> {
    let programs = frame.programs;
    match fill.kind {
        FillKind::Constant => constant_rows(programs.code, fill, rows),
        FillKind::Rows => {
            let input = input_at(programs.code, fill.inputs)?;
            input_piped(frame, uses, table, input, rows, kept)
        }
        FillKind::Match => {
            let FillKept { work, reached } = kept;
            let wrong = FilledError::Code {
                named: MissingCode::Column,
            };
            let left = input_at(programs.code, fill.inputs)?;
            let right = input_at(programs.code, left.after)?;
            let left_side = MatchSide {
                keys: left.keys,
                order: left.order.ok_or(wrong.clone())?,
            };
            let right_side = MatchSide {
                keys: right.keys,
                order: right.order.ok_or(wrong)?,
            };
            let mut gathered = [
                frame.store.inputs.pop().unwrap_or_default(),
                frame.store.inputs.pop().unwrap_or_default(),
            ];
            let mut piped = Ok(());
            for (input, into) in [left, right].into_iter().zip(gathered.iter_mut()) {
                if piped.is_ok() {
                    piped = input_width(frame, input).and_then(|width| {
                        clear_rows(into, width);
                        input_piped(
                            frame,
                            uses,
                            table,
                            input,
                            into,
                            FillKept {
                                work: &mut *work,
                                reached: &mut *reached,
                            },
                        )
                    });
                }
            }
            let mut room = std::mem::take(&mut frame.store.room);
            let [gathered_left, gathered_right] = gathered;
            let done = piped.and_then(|()| {
                match_rows(
                    &gathered_left,
                    &gathered_right,
                    left_side,
                    right_side,
                    &mut room.matching,
                    rows,
                )
            });
            work.read = Count(
                work.read
                    .0
                    .saturating_add(gathered_left.count.0)
                    .saturating_add(gathered_right.count.0),
            );
            work.given = Count(work.given.0.saturating_add(rows.count.0));
            frame.store.room = room;
            frame.store.inputs.push(gathered_left);
            frame.store.inputs.push(gathered_right);
            done
        }
        FillKind::Count | FillKind::Sorted | FillKind::Ends => {
            let FillKept { work, reached } = kept;
            let input = input_at(programs.code, fill.inputs)?;
            let mut gathered = frame.store.inputs.pop().unwrap_or_default();
            clear_rows(&mut gathered, input_width(frame, input)?);
            let piped = input_piped(
                frame,
                uses,
                table,
                input,
                &mut gathered,
                FillKept {
                    work: &mut *work,
                    reached,
                },
            );
            let mut room = std::mem::take(&mut frame.store.room);
            let done =
                piped.and_then(|()| grouped_rows(fill, &gathered, input, &mut room, rows, work));
            frame.store.room = room;
            frame.store.inputs.push(gathered);
            done
        }
    }
}

/// Makes `table` for the file if no read of the file made it: its fill runs once. The table's slot then holds its rows,
/// the work of its own fill and what the fill reached directly.
///
/// # Errors
///
/// A source its fill reads could not be read; the fill is one the fill side does not run yet; or the table is read while
/// its own fill runs.
pub(crate) fn made_now(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
) -> Result<MadeNow, FilledError> {
    let stamp = frame.store.stamp;
    let fill = fill_at(frame.programs.code, table)?;
    let slot = slot_mut(frame, table).ok_or(FilledError::Code {
        named: MissingCode::Table,
    })?;
    if slot.stamp == stamp {
        match slot.state {
            MadeState::Made => return Ok(MadeNow::Held),
            MadeState::Making => {
                return Err(FilledError::Unbuilt {
                    unbuilt: UnbuiltFill::Written,
                });
            }
            MadeState::Unmade => {}
        }
    }
    slot.stamp = stamp;
    slot.state = MadeState::Making;
    let mut rows = std::mem::take(&mut slot.rows);
    let mut reached = std::mem::take(&mut slot.reached);
    clear_rows(&mut rows, fill.width);
    reached.clear();
    let mut work = FillWork::default();
    let done = filled_now(
        frame,
        uses,
        table,
        fill,
        &mut rows,
        FillKept {
            work: &mut work,
            reached: &mut reached,
        },
    );
    work.made = rows.count;
    if let Some(slot) = slot_mut(frame, table) {
        slot.rows = rows;
        slot.reached = reached;
        slot.work = work;
        slot.state = match done {
            Ok(()) => MadeState::Made,
            Err(_) => MadeState::Unmade,
        };
    }
    done.map(|()| MadeNow::Made)
}
