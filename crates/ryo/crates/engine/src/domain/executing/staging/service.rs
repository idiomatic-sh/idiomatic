//! The services of the `staging` part: every row of every body of an `Or` word, each as the cells of the word's outputs in
//! order, staged once per ask, or once per key when the word keeps its answer.

use crate::domain::code::constants::STORAGE;
use crate::domain::code::controlling::service::or_of;
use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{CodeWord, PoolAt, Short, WordSlot};
use crate::domain::executing::bodies::service::{kept_key, pattern, table_rows};
use crate::domain::executing::bodies::types::CallUse;
use crate::domain::indexes::types::RowRun;
use crate::domain::made_tables::reading::service::rows_at;
use crate::domain::made_tables::sources::service::{cell_of, handle_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::memos::service::{asked, keep, row_count, rows};
use crate::domain::memos::types::MemoId;
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::operands::service::listed;
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use crate::domain::steps::types::MadeTableId;
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
            bound,
            formals,
        } => {
            let answer = pattern(
                frame,
                CallUse {
                    entry,
                    site: memo,
                    unit,
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
            for place in found.run.start.0..found.run.end.0 {
                let run = RowRun {
                    start: Count(place),
                    end: Count(place.saturating_add(1)),
                };
                let Some(row) = rows_at(frame.fill.store, run).first().copied() else {
                    continue;
                };
                for output in outputs {
                    // The pair of this output among the body's reads names the column of the table's row.
                    let column = (0..collected.0)
                        .map(|number| {
                            usize::try_from(reads.0.wrapping_add(number.wrapping_mul(2)))
                                .unwrap_or(usize::MAX)
                        })
                        .find(|pair| programs.code.pool.get(pair.saturating_add(1)) == Some(output))
                        .and_then(|pair| programs.code.pool.get(pair))
                        .map_or(u16::MAX, |held| u16::try_from(held.0).unwrap_or(u16::MAX));
                    let value =
                        cell_of(frame.fill, source, Short(column), handle_of(row, found.tag));
                    frame.state.staged.push(value);
                }
            }
            Count(found.run.end.0.saturating_sub(found.run.start.0))
        }
    }
}

/// Stages the rows of every body of an `Or` word, each as the cells of its outputs in order, once per key when the word
/// keeps its answer, and leaves where they start and how many they are in the state for the word's item.
pub(crate) fn staged(frame: &mut StepFrame<'_, '_>, word: CodeWord) {
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
            let (mut at, mut total) = (asking.records, 0_usize);
            for _ in 0..asking.bodies.0 {
                let Some(body) = body_at(&frame.fill.programs.code.pool, at) else {
                    break;
                };
                at = body.after;
                let outputs = listed(&frame.fill.programs.code.pool, asking.outputs);
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
