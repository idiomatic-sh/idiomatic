//! The services of the `staging` part: every row of every body of an `Or` word, each as the cells of the word's outputs in
//! order, staged once per ask, or once per key when the word keeps its answer.

use crate::domain::code::constants::STORAGE;
use crate::domain::code::controlling::service::or_of;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{CodeEntry, CodeWord, PoolAt, Short, WordSlot};
use crate::domain::executing::bodies::service::{kept_key, pattern, table_rows};
use crate::domain::executing::bodies::types::CallUse;
use crate::domain::indexes::reading::service::cell_at;
use crate::domain::made_tables::reading::service::{rows_at, view_of};
use crate::domain::made_tables::sources::service::handle_of;
use crate::domain::made_tables::types::ReadSource;
use crate::domain::memos::service::{asked, keep, row_count, rows};
use crate::domain::memos::types::{CellSpan, MemoId};
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::operands::service::listed;
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use crate::domain::stages::service::row_of;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Stages the rows of one body, each as the cells of the asking word's output registers in order, and answers how many.
/// A table-form body's row gives, for each output register, the column its record pairs with that register.
pub(crate) fn body_staged(
    frame: &mut StepFrame<'_, '_>,
    record: BodyRecord,
    outputs: &[WordSlot],
) -> Count {
    match record {
        BodyRecord::Inline { entry } => {
            let base = frame.state.yielded.len();
            let count = frame.ports.run(frame, entry, RunMode::Collect);
            let LoopState {
                yielded, staged, ..
            } = &mut *frame.state;
            staged.extend(yielded.drain(base.min(yielded.len())..));
            Count(usize::try_from(count.0).unwrap_or(usize::MAX))
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
            let LoopState { memos, staged, .. } = &mut *frame.state;
            staged.extend_from_slice(rows(memos, answer));
            row_count(memos, answer)
        }
        BodyRecord::Table {
            index,
            bound,
            values,
            collected,
            reads,
        } => {
            let found = table_rows(frame, index, bound, values);
            let programs = frame.fill.programs;
            let table = programs
                .code
                .indexes
                .get(usize::try_from(index.0).unwrap_or(usize::MAX))
                .map_or(0, |code| code.number.0);
            let source = ReadSource::Made(MadeTableId(table));
            // A row of the staged rows is the cells of the outputs in order. Each output's column is found among the
            // body's reads and asked of the seam once, and then read for every row found.
            let width = outputs.len();
            let first = frame.state.staged.len();
            let found_rows = rows_at(frame.fill.store, found.run).len();
            frame.state.staged.resize(
                first.saturating_add(found_rows.saturating_mul(width)),
                CellWord(0),
            );
            for (number, output) in outputs.iter().enumerate() {
                // The pair of this output among the body's reads names the column of the table's row.
                let column = (0..collected.0)
                    .map(|pair| {
                        usize::try_from(reads.0.wrapping_add(pair.wrapping_mul(2)))
                            .unwrap_or(usize::MAX)
                    })
                    .find(|pair| programs.code.pool.get(pair.saturating_add(1)) == Some(output))
                    .and_then(|pair| programs.code.pool.get(pair))
                    .map_or(u16::MAX, |held| u16::try_from(held.0).unwrap_or(u16::MAX));
                let held = view_of(frame.fill.store, source, Short(column));
                let handles = rows_at(frame.fill.store, found.run);
                let cells = frame
                    .state
                    .staged
                    .iter_mut()
                    .skip(first.saturating_add(number));
                for (row, staged) in handles.iter().zip(cells.step_by(width.max(1))) {
                    *staged = cell_at(held, row_of(handle_of(*row, found.tag)));
                }
            }
            Count(found.run.end.0.saturating_sub(found.run.start.0))
        }
    }
}

/// Stages the rows of every body of an `Or` word, each as the cells of its outputs in order, once per key when the word
/// keeps its answer, and leaves where they start and how many they are in the state for the word's item.
pub(crate) fn staged(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) {
    let asking = or_of(word);
    let first = frame.state.staged.len();
    let outputs = listed(&frame.fill.programs.code.pool, asking.outputs).len();
    let keys = PoolAt(
        asking
            .outputs
            .0
            .wrapping_add(u32::try_from(outputs.saturating_add(1)).unwrap_or(u32::MAX)),
    );
    let keeps = asking.flag.0 & STORAGE.0 != 0;
    let count = listed(&frame.fill.programs.code.pool, keys).len();
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
            // A word the decoded code holds no list for has no body.
            let bodies = match frame
                .state
                .decoded
                .lists
                .get(usize::try_from(at.0).unwrap_or(usize::MAX))
            {
                Some(list) => *list,
                None => CellSpan::default(),
            };
            let outputs = listed(&frame.fill.programs.code.pool, asking.outputs);
            let mut total = 0_usize;
            for number in 0..bodies.length.0 {
                let place = bodies.start.0.saturating_add(number);
                let Some(body) = frame.state.decoded.or_bodies.get(place).copied() else {
                    break;
                };
                total = total.saturating_add(body_staged(frame, body.record, outputs).0);
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
