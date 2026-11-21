//! The services of the `ordered` part: the rows of a sorted table nearest a bound in a key's group, staged for the
//! `Latest` word's item. The table is made and its index built by the seam's read by key; which rows of the group are
//! nearest is the fill side's kernel (`stages::service::latest`, the operators crate's meaning), run for a batch of one.

use crate::domain::code::anchoring::service::latest_of;
use crate::domain::code::constants::{BOUND_CONSTANT, SIDE};
use crate::domain::code::small_codes::service::side_of;
use crate::domain::code::types::{CodeWord, ConstantNumber, PoolAt, WordFlag, WordSlot};
use crate::domain::executing::bodies::service::key_pushed;
use crate::domain::executing::sources::service::keyed;
use crate::domain::executing::walks::service::{lent, taken_back};
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::reading::service::{index_at, latest_site, site_at};
use crate::domain::made_tables::sources::service::table_of;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, constant};
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use crate::domain::stages::service::latest as nearest_rows;
use crate::domain::stages::types::{BatchKey, BatchValue};
use crate::domain::steps::types::{MadeTableId, Register};
use ryo_relations::domain::relations::types::Count;

/// Stages the handles of the rows a `Latest` word reads: of the group its key names in its sorted table, every row tied
/// for nearest its bound on the word's side. Leaves where they are in the state for the word's item; no row when the
/// table has none for the file or the key no group. A word whose record or side the code does not hold reads no row and
/// is counted as not run.
pub(crate) fn latest(frame: &mut StepFrame<'_, '_>, word: CodeWord) {
    let asking = latest_of(word);
    let code = frame.fill.programs.code;
    frame.state.answer = BodyAnswer {
        first: Count(frame.state.staged.len()),
        rows: Count(0),
    };
    let site = latest_site(code, asking.operands);
    let table = site
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|site| site_at(code, *site));
    let (Ok(site), Ok(table), Some(side)) =
        (site, table, side_of(WordFlag(asking.flag.0 & SIDE.0)))
    else {
        frame.state.work.unbuilt = InstructionCount(frame.state.work.unbuilt.0.saturating_add(1));
        return;
    };
    let bound = if asking.flag.0 & BOUND_CONSTANT.0 == 0 {
        cell(frame, Register(asking.bound.0))
    } else {
        constant(frame, ConstantNumber(asking.bound.0))
    };
    let key = key_pushed(
        frame,
        WordSlot(u32::from(asking.keys.0)),
        PoolAt(asking.operands.0.saturating_add(1)),
    );
    // The read by key makes the table and builds the index when its need holds: no group, no row.
    let group = keyed(frame, site, key.start, key.from);
    if group.run.end.0 > group.run.start.0
        && let Some(sorted) = table_of(frame.fill.store, MadeTableId(table.number.0))
    {
        let first = lent(frame.state, group.tag);
        let LoopState {
            key: cells, fan, ..
        } = &mut *frame.state;
        let _all = nearest_rows(
            index_at(frame.fill.store, site),
            &sorted.rows.orders,
            BatchKey {
                cells: cells.get_mut(key.start.0..).unwrap_or_default(),
                parts: &[],
            },
            BatchValue::Constant(bound),
            side,
            Count(1),
            fan,
        );
        frame.state.answer = taken_back(frame.state, first);
    }
    frame.state.key.truncate(key.start.0);
}
