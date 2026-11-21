//! The services of the `bodies` part: what a word over a body asks of it. An inline body is run in place; a pattern in
//! call form is asked through the one memo, by its site and the cells of its bound columns, and its program runs once per
//! key; a pattern in table form is a read of its table's index, through the fill side's seam. The loop is reached through
//! the frame's ports, so a body runs through the one loop.

use super::types::{CallUse, PushedKey};
use crate::domain::code::constants::STORAGE;
use crate::domain::code::controlling::service::exists_of;
use crate::domain::code::records::service::{body_at, operand_at};
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::records::types::{BodyRecord, ReadBody};
use crate::domain::code::types::{CodeEntry, CodeWord, IndexSite, Op, PoolAt, WordSlot};
use crate::domain::executing::asks::service::definition_needed;
use crate::domain::executing::budget::service::{begin_unit, end_unit, reach};
use crate::domain::executing::constants::NO_ANSWER;
use crate::domain::executing::measuring::service::{entered, evaluated, left, used};
use crate::domain::executing::sources::service::keyed;
use crate::domain::made_tables::types::{KeyFrom, PatternAsk};
use crate::domain::memos::service::{asked, keep, row_count};
use crate::domain::memos::types::{ChargedUnit, MemoId};
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::operands::service::{cell, operand, set};
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

/// The answer of a pattern in call form for the key the loop's key room holds from `start` on: the kept one, or the
/// pattern's program run once for this key, its yielded rows kept. The key's cells arrive in the formals. The key is taken
/// off the room.
fn pattern_keyed(frame: &mut StepFrame<'_, '_>, body: CallUse, start: Count) -> MemoId {
    used(frame.state, body.unit);
    let kept = asked(
        &frame.state.memos,
        body.site,
        frame.state.key.get(start.0..).unwrap_or_default(),
    );
    if let Some(answer) = kept {
        frame.state.key.truncate(start.0);
        reach(frame, ChargedUnit::Answer(answer));
        return answer;
    }
    let first = parts_start(frame.state, body);
    for number in 0..usize::try_from(body.bound.0).unwrap_or(usize::MAX) {
        let part = first.and_then(|at| frame.state.decoded.parts.get(at.0.saturating_add(number)));
        let Some(formal) = part.map(|held| held.formal) else {
            continue;
        };
        let value = match frame.state.key.get(start.0.saturating_add(number)) {
            Some(held) => *held,
            None => CellWord(0),
        };
        set(frame, formal, value);
    }
    let base = frame.state.yielded.len();
    let caller = begin_unit(frame.state);
    entered(frame.state, body.unit);
    let count = frame.ports.run(frame, body.entry, RunMode::Collect);
    left(frame.state);
    let given = Count(usize::try_from(count.0).unwrap_or(usize::MAX));
    evaluated(frame.state, body.unit, Count(1), given);
    let LoopState {
        memos,
        key,
        yielded,
        ..
    } = &mut *frame.state;
    let answer = keep(
        memos,
        body.site,
        key.get(start.0..).unwrap_or_default(),
        yielded.get(base..).unwrap_or_default(),
        Count(usize::try_from(count.0).unwrap_or(usize::MAX)),
    );
    yielded.truncate(base);
    key.truncate(start.0);
    end_unit(frame.state, caller, answer, body.unit);
    reach(frame, ChargedUnit::Answer(answer));
    answer
}

/// The answer of a pattern in call form for the caller's values, which are the operands paired with its formals; no
/// answer, and no ask, in a file where the pattern's definition cannot have a row.
pub(crate) fn pattern(frame: &mut StepFrame<'_, '_>, body: CallUse) -> MemoId {
    if definition_needed(frame, body.need) == NeedVerdict::Empty {
        return NO_ANSWER;
    }
    let start = frame.state.key.len();
    let first = parts_start(frame.state, body);
    let mut from = KeyFrom::Constants;
    for number in 0..usize::try_from(body.bound.0).unwrap_or(usize::MAX) {
        let part = first.and_then(|at| {
            frame
                .state
                .decoded
                .parts
                .get(at.0.saturating_add(number))
                .copied()
        });
        from = part.map_or(from, |held| held.from);
        let value = match part.and_then(|held| held.value) {
            Some(held) => operand(frame, held),
            None => CellWord(0),
        };
        frame.state.key.push(value);
    }
    // A key of constants alone names one answer a file: it is the site's, found again with no hash.
    let site = usize::try_from(body.site.0).unwrap_or(usize::MAX);
    if from == KeyFrom::Constants
        && let Some(answer) = frame.state.site_answers.get(site).copied().flatten()
    {
        frame.state.key.truncate(start);
        used(frame.state, body.unit);
        reach(frame, ChargedUnit::Answer(answer));
        return answer;
    }
    let answer = pattern_keyed(frame, body, Count(start));
    if from == KeyFrom::Constants {
        let answers = &mut frame.state.site_answers;
        if answers.len() <= site {
            answers.resize(site.saturating_add(1), None);
        }
        if let Some(slot) = answers.get_mut(site) {
            *slot = Some(answer);
        }
    }
    answer
}

/// Where the parts of a call-form body's ask start in the decoded code; none for a body it does not hold.
fn parts_start(state: &LoopState, body: CallUse) -> Option<Count> {
    let place = usize::try_from(body.formals.0).unwrap_or(usize::MAX);
    let held = state.decoded.parts_at.get(place)?;
    held.0.checked_sub(1).map(Count)
}

/// The answer of a pattern in call form for a key the fill side hands over: what a `Use` stage asks back, once per distinct
/// key of its batch. The one memo makes the pattern's program run once per distinct key of the file.
pub(crate) fn asked_pattern(frame: &mut StepFrame<'_, '_>, asked_for: PatternAsk<'_>) -> MemoId {
    let record = body_at(&frame.fill.programs.code.pool, asked_for.body).map(|body| body.record);
    if let Some(BodyRecord::Call { need, .. }) = record
        && definition_needed(frame, need) == NeedVerdict::Empty
    {
        return NO_ANSWER;
    }
    let start = frame.state.key.len();
    frame.state.key.extend_from_slice(asked_for.key);
    match record {
        Some(BodyRecord::Call {
            entry,
            memo,
            unit,
            need,
            bound,
            formals,
        }) => pattern_keyed(
            frame,
            CallUse {
                entry,
                site: memo,
                unit,
                need,
                bound,
                formals,
            },
            Count(start),
        ),
        Some(BodyRecord::Inline { .. } | BodyRecord::Table { .. }) | None => {
            // A `Use` stage's body is a pattern in call form: anything else has no row, kept so under the asked site.
            let LoopState { memos, key, .. } = &mut *frame.state;
            let answer = keep(
                memos,
                asked_for.memo,
                key.get(start..).unwrap_or_default(),
                &[],
                Count(0),
            );
            key.truncate(start);
            answer
        }
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
