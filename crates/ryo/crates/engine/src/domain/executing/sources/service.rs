//! The services of the `sources` part: a word's rows asked of the fill side. A made table is asked only when its own need
//! holds in the file; a fault the fill side answers is kept as the scan's (the first of the file), a fill it does not run
//! yet is counted, and either way the word has no row. A source read is a unit the running rule reached: the budget is
//! told after each read. A need's own reads are not: a decision is made once per file, by whichever rule asks first, so
//! charging them would make a rule's charge depend on the rules before it.

use super::types::FileNeeds;
use crate::domain::code::types::{IndexSite, WordSlot};
use crate::domain::executing::budget::service::reach;
use crate::domain::executing::constants::KEY_CELLS;
use crate::domain::executing::measuring::service::{entered, left, used};
use crate::domain::executing::presence::service::consulted;
use crate::domain::executing::types::Asker;
use crate::domain::filled::errors::FilledError;
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::reading::service::site_source;
use crate::domain::made_tables::sources::service::{matched, scanned, table_of};
use crate::domain::made_tables::types::{KeyFrom, Reached, ReadSource};
use crate::domain::memos::types::ChargedUnit;
use crate::domain::needs::service::decided;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::scratch::types::{FileFault, LoopState, SeamRows};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Keeps what the fill side answered instead of rows.
pub(crate) fn faulted(state: &mut LoopState, fault: FilledError) {
    match fault {
        FilledError::Unbuilt { .. } => {
            state.work.unbuilt = InstructionCount(state.work.unbuilt.0.saturating_add(1));
        }
        FilledError::Refused { .. }
        | FilledError::Partial { .. }
        | FilledError::Code { .. }
        | FilledError::Meaning { .. } => {
            if state.fault.is_none()
                && let Some(rule) = state.rule
            {
                state.fault = Some(FileFault { rule, fault });
            }
        }
    }
}

/// The unit of the budget a source is: none for a table read while its fill writes it, which is that fill's own.
const fn unit_of(source: ReadSource) -> Option<ChargedUnit> {
    match source {
        ReadSource::Nodes(_) => Some(ChargedUnit::Fill(Reached::Nodes)),
        ReadSource::Filled(relation) => Some(ChargedUnit::Fill(Reached::Relation(relation))),
        ReadSource::Made(table) => Some(ChargedUnit::Fill(Reached::Table(table))),
        ReadSource::Written(_) => None,
    }
}

/// A read is about to reach a made table: a use of its unit, and, when the table is not made for the file yet, the time
/// of the read is the table's. Answers whether the unit was entered, to be left after the read.
fn table_read(frame: &mut StepFrame<'_, '_>, table: MadeTableId) -> Option<WordSlot> {
    let unit = frame
        .fill
        .programs
        .code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        .map(|code| code.unit)?;
    used(frame.state, unit);
    table_of(frame.fill.store, table).is_none().then(|| {
        entered(frame.state, unit);
        unit
    })
}

/// Whether a made table may have a row in the file: its own need, decided once per file.
pub(crate) fn table_needed(frame: &mut StepFrame<'_, '_>, table: MadeTableId) -> NeedVerdict {
    let programs = frame.fill.programs;
    match programs
        .code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
    {
        Some(code) => {
            let verdict = decided(&mut FileNeeds(frame), programs.code, code.need);
            consulted(frame, code.need);
            verdict
        }
        None => NeedVerdict::Empty,
    }
}

/// Every row of a source, made or filled on this first read of the file: a made table when its need holds.
pub(crate) fn whole(frame: &mut StepFrame<'_, '_>, source: ReadSource) -> SeamRows {
    if let ReadSource::Made(table) = source
        && table_needed(frame, table) == NeedVerdict::Empty
    {
        return SeamRows::default();
    }
    // A plain worker asks nothing of the measuring hooks: not even whether the table is made.
    let timed = match source {
        ReadSource::Made(table) if frame.state.measure.is_some() => table_read(frame, table),
        ReadSource::Made(_)
        | ReadSource::Nodes(_)
        | ReadSource::Filled(_)
        | ReadSource::Written(_) => None,
    };
    let asked = scanned(
        frame.fill,
        &mut Asker {
            state: &mut *frame.state,
            found: &mut *frame.found,
            ports: frame.ports,
        },
        source,
    );
    if timed.is_some() {
        left(frame.state);
    }
    match asked {
        Ok(rows) => {
            if let Some(unit) = unit_of(source) {
                reach(frame, unit);
            }
            SeamRows {
                run: rows.rows,
                tag: rows.tag,
            }
        }
        Err(fault) => {
            faulted(frame.state, fault);
            SeamRows::default()
        }
    }
}

/// The rows of a site's index for the key the loop's key room holds from `start` on, the site's source made first: a made
/// table when its need holds. The key is copied out of the room, since a pattern the fill side asks back uses the room.
pub(crate) fn keyed(
    frame: &mut StepFrame<'_, '_>,
    site: IndexSite,
    start: Count,
    from: KeyFrom,
) -> SeamRows {
    let over = frame
        .fill
        .programs
        .code
        .indexes
        .get(usize::try_from(site.0).unwrap_or(usize::MAX))
        .copied();
    if let Some(code) = over
        && code.source.0 == 2
        && table_needed(frame, MadeTableId(code.number.0)) == NeedVerdict::Empty
    {
        return SeamRows::default();
    }
    let mut short = [CellWord(0); KEY_CELLS];
    let cells = frame.state.key.get(start.0..).unwrap_or_default();
    let long: Vec<CellWord> = if cells.len() > KEY_CELLS {
        cells.to_vec()
    } else {
        Vec::new()
    };
    let width = cells.len();
    for (slot, cell) in short.iter_mut().zip(cells) {
        *slot = *cell;
    }
    let key = if width > KEY_CELLS {
        long.as_slice()
    } else {
        short.get(..width).unwrap_or_default()
    };
    let timed = over
        .filter(|code| code.source.0 == 2 && frame.state.measure.is_some())
        .and_then(|code| table_read(frame, MadeTableId(code.number.0)));
    let asked = matched(
        frame.fill,
        &mut Asker {
            state: &mut *frame.state,
            found: &mut *frame.found,
            ports: frame.ports,
        },
        site,
        key,
        from,
    );
    if timed.is_some() {
        left(frame.state);
    }
    match asked {
        Ok(rows) => {
            // The site's source is what the rule reached: an index holds no rows of its own.
            let source = over.and_then(|code| match site_source(frame.fill.programs.code, code) {
                Ok(source) => Some(source),
                Err(_) => None,
            });
            if let Some(unit) = source.and_then(unit_of) {
                reach(frame, unit);
            }
            SeamRows {
                run: rows.rows,
                tag: rows.tag,
            }
        }
        Err(fault) => {
            faulted(frame.state, fault);
            SeamRows::default()
        }
    }
}
