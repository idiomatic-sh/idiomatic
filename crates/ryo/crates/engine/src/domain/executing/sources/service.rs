//! The services of the `sources` part: a word's rows asked of the fill side. A made table is asked only when its own need
//! holds in the file; a fault the fill side answers is kept as the scan's (the first of the file), a fill it does not run
//! yet is counted, and either way the word has no row.

use super::types::FileNeeds;
use crate::domain::code::types::IndexSite;
use crate::domain::executing::constants::KEY_CELLS;
use crate::domain::executing::types::Asker;
use crate::domain::filled::errors::FilledError;
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::sources::service::{matched, scanned};
use crate::domain::made_tables::types::{KeyFrom, ReadSource};
use crate::domain::needs::service::decided;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::scratch::types::{FileFault, LoopState, SeamRows};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Keeps what the fill side answered instead of rows.
fn faulted(state: &mut LoopState, fault: FilledError) {
    match fault {
        FilledError::Unbuilt { .. } => {
            state.work.unbuilt = InstructionCount(state.work.unbuilt.0.saturating_add(1));
        }
        FilledError::Refused { .. } | FilledError::Partial { .. } | FilledError::Code { .. } => {
            if state.fault.is_none()
                && let Some(rule) = state.rule
            {
                state.fault = Some(FileFault { rule, fault });
            }
        }
    }
}

/// Whether a made table may have a row in the file: its own need, decided once per file.
pub(crate) fn table_needed(frame: &mut StepFrame<'_, '_>, table: MadeTableId) -> NeedVerdict {
    let programs = frame.fill.programs;
    match programs
        .code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
    {
        Some(code) => decided(&mut FileNeeds(frame), &programs.code, code.need),
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
    let asked = scanned(
        frame.fill,
        &mut Asker {
            state: &mut *frame.state,
            found: &mut *frame.found,
            ports: frame.ports,
        },
        source,
    );
    match asked {
        Ok(rows) => SeamRows {
            run: rows.rows,
            tag: rows.tag,
        },
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
    match asked {
        Ok(rows) => SeamRows {
            run: rows.rows,
            tag: rows.tag,
        },
        Err(fault) => {
            faulted(frame.state, fault);
            SeamRows::default()
        }
    }
}
