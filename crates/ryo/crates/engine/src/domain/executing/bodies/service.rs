//! The services of the `bodies` part: what a word over a body asks of it. An inline body is run in place; a pattern in
//! call form is asked through the one memo, by its site and the cells of its bound columns, and its program runs once per
//! key; a pattern in table form is a read of its table's index, which is the fill side's seam and not built here yet. The
//! loop is handed in, so a body runs through the one loop.

use crate::domain::code::constants::STORAGE;
use crate::domain::code::controlling::service::{exists_of, or_of};
use crate::domain::code::records::service::{body_at, operand_at};
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{CodeEntry, CodeWord, MemoSite, PoolAt, WordSlot};
use crate::domain::executing::types::{RunMode, Runner};
use crate::domain::findings::types::InstructionCount;
use crate::domain::memos::service::{asked, keep, row_count, rows};
use crate::domain::memos::types::MemoId;
use crate::domain::programs::collections::instructions::interface::StepFrame;
use crate::domain::programs::operands::service::{cell, listed, operand, set};
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The slot `count` slots after a place of the pool; nought past the pool's end.
fn slot(frame: &StepFrame<'_, '_>, at: PoolAt, count: WordSlot) -> WordSlot {
    match frame
        .programs
        .code
        .pool
        .get(usize::try_from(at.0.wrapping_add(count.0)).unwrap_or(usize::MAX))
    {
        Some(held) => *held,
        None => WordSlot(0),
    }
}

/// The answer of a pattern in call form for the caller's values: the kept one, or the pattern's program run once for this
/// key, its yielded rows kept. `formals` is where the pairs of a formal register and the caller's operand start: the
/// caller's cells are the key, and arrive in the formals.
pub(crate) fn pattern(
    frame: &mut StepFrame<'_, '_>,
    entry: CodeEntry,
    site: MemoSite,
    bound: WordSlot,
    formals: PoolAt,
    run: Runner,
) -> MemoId {
    let start = frame.state.key.len();
    for number in 0..bound.0 {
        let at = PoolAt(
            formals
                .0
                .wrapping_add(number.wrapping_mul(3))
                .wrapping_add(1),
        );
        let value = operand_at(&frame.programs.code.pool, at)
            .map_or(CellWord(0), |held| operand(frame, held));
        frame.state.key.push(value);
    }
    let kept = asked(
        &frame.state.memos,
        site,
        frame.state.key.get(start..).unwrap_or_default(),
    );
    if let Some(answer) = kept {
        frame.state.key.truncate(start);
        return answer;
    }
    for number in 0..bound.0 {
        let formal = slot(frame, formals, WordSlot(number.wrapping_mul(3)));
        let value = match frame
            .state
            .key
            .get(start.saturating_add(usize::try_from(number).unwrap_or(usize::MAX)))
        {
            Some(held) => *held,
            None => CellWord(0),
        };
        set(frame, Register(formal.0), value);
    }
    let base = frame.state.yielded.len();
    let count = run(frame, entry, RunMode::Collect);
    let LoopState {
        memos,
        key,
        yielded,
        ..
    } = &mut *frame.state;
    let answer = keep(
        memos,
        site,
        key.get(start..).unwrap_or_default(),
        yielded.get(base..).unwrap_or_default(),
        Count(usize::try_from(count.0).unwrap_or(usize::MAX)),
    );
    yielded.truncate(base);
    key.truncate(start);
    answer
}

/// How many rows a body has, as far as an `Exists` asks: none or some.
fn has_row(frame: &mut StepFrame<'_, '_>, at: PoolAt, run: Runner) -> Count {
    let Some(body) = body_at(&frame.programs.code.pool, at) else {
        return Count(0);
    };
    match body.record {
        BodyRecord::Inline { entry } => {
            Count(usize::try_from(run(frame, entry, RunMode::Exists).0).unwrap_or(usize::MAX))
        }
        BodyRecord::Call {
            entry,
            memo,
            bound,
            formals,
        } => {
            let answer = pattern(frame, entry, memo, bound, formals, run);
            row_count(&frame.state.memos, answer)
        }
        BodyRecord::Table { .. } => {
            frame.state.work.unbuilt =
                InstructionCount(frame.state.work.unbuilt.0.saturating_add(1));
            Count(0)
        }
    }
}

/// Pushes onto the key room the cells of `count` registers named in the pool from `registers` on; answers where the key
/// starts.
fn kept_key(frame: &mut StepFrame<'_, '_>, registers: PoolAt, count: WordSlot) -> Count {
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
pub(crate) fn exists(frame: &mut StepFrame<'_, '_>, word: CodeWord, run: Runner) {
    let asking = exists_of(word);
    if asking.flag.0 & STORAGE.0 == 0 {
        let found = has_row(frame, asking.body, run);
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
            let found = has_row(frame, asking.body, run);
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

/// Stages the rows of one body of an `Or`, each as the cells of the word's outputs in order, and answers how many.
fn body_staged(frame: &mut StepFrame<'_, '_>, record: BodyRecord, run: Runner) -> Count {
    match record {
        BodyRecord::Inline { entry } => {
            let base = frame.state.yielded.len();
            let count = run(frame, entry, RunMode::Collect);
            let LoopState {
                yielded, staged, ..
            } = &mut *frame.state;
            staged.extend(yielded.drain(base.min(yielded.len())..));
            Count(usize::try_from(count.0).unwrap_or(usize::MAX))
        }
        BodyRecord::Call {
            entry,
            memo,
            bound,
            formals,
        } => {
            let answer = pattern(frame, entry, memo, bound, formals, run);
            let LoopState { memos, staged, .. } = &mut *frame.state;
            staged.extend_from_slice(rows(memos, answer));
            row_count(memos, answer)
        }
        BodyRecord::Table { .. } => {
            frame.state.work.unbuilt =
                InstructionCount(frame.state.work.unbuilt.0.saturating_add(1));
            Count(0)
        }
    }
}

/// Stages the rows of every body of an `Or` word, each as the cells of its outputs in order, once per key when the word
/// keeps its answer, and leaves where they start and how many they are in the state for the word's item.
pub(crate) fn staged(frame: &mut StepFrame<'_, '_>, word: CodeWord, run: Runner) {
    let asking = or_of(word);
    let first = frame.state.staged.len();
    let outputs = listed(&frame.programs.code.pool, asking.outputs).len();
    let keys = PoolAt(
        asking
            .outputs
            .0
            .wrapping_add(u32::try_from(outputs.saturating_add(1)).unwrap_or(u32::MAX)),
    );
    let keeps = asking.flag.0 & STORAGE.0 != 0;
    let count = listed(&frame.programs.code.pool, keys).len();
    let start = if keeps {
        kept_key(
            frame,
            PoolAt(keys.0.wrapping_add(1)),
            WordSlot(u32::try_from(count).unwrap_or(u32::MAX)),
        )
    } else {
        Count(frame.state.key.len())
    };
    let kept = if keeps {
        asked(
            &frame.state.memos,
            asking.memo,
            frame.state.key.get(start.0..).unwrap_or_default(),
        )
    } else {
        None
    };
    let total = match kept {
        Some(answer) => {
            let LoopState { memos, staged, .. } = &mut *frame.state;
            staged.extend_from_slice(rows(memos, answer));
            row_count(memos, answer)
        }
        None => {
            let (mut at, mut total) = (asking.records, 0_usize);
            for _ in 0..asking.bodies.0 {
                let Some(body) = body_at(&frame.programs.code.pool, at) else {
                    break;
                };
                at = body.after;
                total = total.saturating_add(body_staged(frame, body.record, run).0);
            }
            if keeps {
                let LoopState {
                    memos, staged, key, ..
                } = &mut *frame.state;
                let _kept: MemoId = keep(
                    memos,
                    asking.memo,
                    key.get(start.0..).unwrap_or_default(),
                    staged.get(first..).unwrap_or_default(),
                    Count(total),
                );
            }
            Count(total)
        }
    };
    frame.state.key.truncate(start.0);
    frame.state.answer = BodyAnswer {
        first: Count(first),
        rows: total,
    };
}
