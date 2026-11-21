//! The services of the `bodies` part: what a word over a body asks of it. An inline body is run in place; a pattern in
//! call form is asked through the one memo, by its site and the cells of its bound columns, and its program runs once per
//! key; a pattern in table form is a read of its table's index, through the fill side's seam. The loop is reached through
//! the frame's ports, so a body runs through the one loop.

use super::types::{CallUse, PushedKey};
use crate::domain::code::constants::STORAGE;
use crate::domain::code::controlling::service::exists_of;
use crate::domain::code::records::service::operand_at;
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::records::types::{BodyRecord, ReadBody};
use crate::domain::code::types::{CodeEntry, CodeWord, IndexSite, Op, PoolAt, WordSlot};
use crate::domain::executing::kept::service::pattern;
use crate::domain::executing::sources::service::keyed;
use crate::domain::made_tables::types::KeyFrom;
use crate::domain::memos::service::{asked, keep, row_count};
use crate::domain::memos::types::MemoId;
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::operands::service::{cell, operand};
use crate::domain::scratch::types::SeamRows;
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The slot `count` slots after a place of the pool; nought past the pool's end.
pub(crate) fn slot(frame: &StepFrame<'_, '_>, at: PoolAt, count: WordSlot) -> WordSlot {
    match frame
        .fill
        .programs
        .code
        .pool
        .get(usize::try_from(at.0.wrapping_add(count.0)).unwrap_or(usize::MAX))
    {
        Some(held) => *held,
        None => WordSlot(0),
    }
}

/// Pushes onto the key room the cells of `count` operands of the pool from `values` on; answers where the key starts,
/// and whether a register is among them: a key that changes with the row.
pub(crate) fn key_pushed(
    frame: &mut StepFrame<'_, '_>,
    count: WordSlot,
    values: PoolAt,
) -> PushedKey {
    let start = frame.state.key.len();
    let mut from = KeyFrom::Constants;
    for number in 0..count.0 {
        let at = PoolAt(values.0.wrapping_add(number.wrapping_mul(2)));
        let held = operand_at(&frame.fill.programs.code.pool, at);
        if matches!(held, Some(PooledOperand::Register(_))) {
            from = KeyFrom::Row;
        }
        let value = held.map_or(CellWord(0), |each| operand(frame, each));
        frame.state.key.push(value);
    }
    PushedKey {
        start: Count(start),
        from,
    }
}

/// The rows of a pattern in table form for the caller's values: the key's stretch of its table's index, through the seam.
pub(crate) fn table_rows(
    frame: &mut StepFrame<'_, '_>,
    index: IndexSite,
    bound: WordSlot,
    values: PoolAt,
) -> SeamRows {
    let key = key_pushed(frame, bound, values);
    let found = keyed(frame, index, key.start, key.from);
    frame.state.key.truncate(key.start.0);
    found
}

/// The body record of the word at a place of the code, as the worker's state holds it: read from the pool when the
/// state was made, not at the step.
#[inline]
pub(crate) fn body_of(frame: &StepFrame<'_, '_>, at: CodeEntry) -> Option<ReadBody> {
    frame
        .state
        .decoded
        .bodies
        .get(usize::try_from(at.0).unwrap_or(usize::MAX))
        .copied()
        .flatten()
}

/// How many rows a body has, as far as an `Exists` or a walk's condition asks: none or some; none of a word that
/// holds no body.
pub(crate) fn has_row(frame: &mut StepFrame<'_, '_>, body: Option<ReadBody>) -> Count {
    let Some(body) = body else {
        return Count(0);
    };
    match body.record {
        BodyRecord::Inline { entry } => {
            // A body that is one `Yield` has a row whatever the candidate: no run is entered for it.
            let first = frame
                .fill
                .programs
                .code
                .words
                .get(usize::try_from(entry.0).unwrap_or(usize::MAX));
            if first.is_some_and(|word| word.op == Op::Yield) {
                return Count(1);
            }
            Count(
                usize::try_from(frame.ports.run(frame, entry, RunMode::Exists).0)
                    .unwrap_or(usize::MAX),
            )
        }
        BodyRecord::Call {
            entry,
            memo,
            unit,
            need,
            bound,
            formals,
        } => {
            let answer = pattern(
                frame,
                CallUse {
                    entry,
                    site: memo,
                    unit,
                    need,
                    bound,
                    formals,
                },
            );
            row_count(&frame.state.memos, answer)
        }
        BodyRecord::Table {
            index,
            bound,
            values,
            ..
        } => {
            let found = table_rows(frame, index, bound, values).run;
            Count(found.end.0.saturating_sub(found.start.0))
        }
    }
}

/// Pushes onto the key room the cells of `count` registers named in the pool from `registers` on; answers where the key
/// starts.
pub(crate) fn kept_key(frame: &mut StepFrame<'_, '_>, registers: PoolAt, count: WordSlot) -> Count {
    let start = frame.state.key.len();
    for number in 0..count.0 {
        let register = slot(frame, registers, WordSlot(number));
        let value = cell(frame, Register(register.0));
        frame.state.key.push(value);
    }
    Count(start)
}

/// Asks the body of an `Exists` word whether it has a row, once per key when the word keeps its answer, and leaves the
/// answer in the state for the word's item.
pub(crate) fn exists(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) {
    let asking = exists_of(word);
    let body = body_of(frame, at);
    if asking.flag.0 & STORAGE.0 == 0 {
        let found = has_row(frame, body);
        frame.state.answer = BodyAnswer {
            first: Count(0),
            rows: found,
        };
        return;
    }
    let start = kept_key(frame, asking.key, WordSlot(u32::from(asking.keys.0)));
    let kept = asked(
        &frame.state.memos,
        asking.memo,
        frame.state.key.get(start.0..).unwrap_or_default(),
    );
    let found = match kept {
        Some(answer) => row_count(&frame.state.memos, answer),
        None => {
            let found = has_row(frame, body);
            let LoopState { memos, key, .. } = &mut *frame.state;
            let _kept: MemoId = keep(
                memos,
                asking.memo,
                key.get(start.0..).unwrap_or_default(),
                &[],
                found,
            );
            found
        }
    };
    frame.state.key.truncate(start.0);
    frame.state.answer = BodyAnswer {
        first: Count(0),
        rows: found,
    };
}
