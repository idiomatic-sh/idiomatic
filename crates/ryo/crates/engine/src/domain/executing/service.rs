//! The services of the `executing` concept: the one loop over a program's words. A word is dispatched to its item; a word
//! that iterates pushes a cursor, and each time a binding is done the innermost cursor binds its next row. No word
//! allocates, and the loop reads no clock: steps and the rows its iterations gave are counted in locals and added to the
//! work once, when the program ends.

use super::bodies::service::exists;
use super::budget::service::reach;
use super::closing::service::closure;
use super::measuring::types::{Uncounted, WordCounter};
use super::ordered::service::latest;
use super::sources::service::{keyed, whole};
use super::staging::service::staged;
use super::walks::service::{axis, nearest};
use crate::domain::code::anchoring::service::{
    each_row_filled_of, each_row_made_of, each_row_node_of,
};
use crate::domain::code::constants::OWN_TREE;
use crate::domain::code::types::{CodeEntry, CodeWord, Op};
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::types::{Reached, ReadSource};
use crate::domain::memos::types::ChargedUnit;
use crate::domain::programs::collections::instructions::axis::item::Axis;
use crate::domain::programs::collections::instructions::comparing::item::Test;
use crate::domain::programs::collections::instructions::each_kind::item::EachKind;
use crate::domain::programs::collections::instructions::each_match::item::EachMatch;
use crate::domain::programs::collections::instructions::each_row::item::EachRow;
use crate::domain::programs::collections::instructions::emit::item::Emit;
use crate::domain::programs::collections::instructions::exists::item::Exists;
use crate::domain::programs::collections::instructions::fork::item::Fork;
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::collections::instructions::latest::item::Latest;
use crate::domain::programs::collections::instructions::let_value::item::LetValue;
use crate::domain::programs::collections::instructions::nearest::item::Nearest;
use crate::domain::programs::collections::instructions::or::item::Or;
use crate::domain::programs::collections::instructions::reached::item::Closure;
use crate::domain::programs::collections::instructions::read::item::Read;
use crate::domain::programs::collections::instructions::yield_row::item::YieldRow;
use crate::domain::programs::frames::types::{RunMode, StepFrame};
use crate::domain::programs::operands::service::after;
use crate::domain::scratch::types::IterationCursor;
use ryo_relations::domain::relations::types::NodeExtent;

/// The word at a place; the word that ends a program past the code's end.
#[inline]
fn word_at(frame: &StepFrame<'_, '_>, at: CodeEntry) -> CodeWord {
    match frame
        .fill
        .programs
        .code
        .words
        .get(usize::try_from(at.0).unwrap_or(usize::MAX))
    {
        Some(word) => *word,
        None => CodeWord::default(),
    }
}

/// Runs one word for the current candidate. A word the executor does not run yet ends the candidate and is counted, so a
/// rule that holds one never finds a row by it.
#[inline]
fn step(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry, mode: RunMode) -> Flow {
    match word.op {
        Op::End => Flow::Back,
        Op::EachKind => {
            reach(frame, ChargedUnit::Fill(Reached::Nodes));
            EachKind::run(frame, word, at)
        }
        Op::EachRowNode | Op::EachRowFilled | Op::EachRowMade => {
            let source = if word.op == Op::EachRowNode {
                let own = each_row_node_of(word).flag.0 & OWN_TREE.0 != 0;
                ReadSource::Nodes(if own {
                    NodeExtent::Own
                } else {
                    NodeExtent::Embedded
                })
            } else if word.op == Op::EachRowFilled {
                ReadSource::Filled(each_row_filled_of(word).relation)
            } else {
                ReadSource::Made(each_row_made_of(word).table)
            };
            frame.state.rows = whole(frame, source);
            EachRow::run(frame, word, at)
        }
        Op::EachMatchNode | Op::EachMatchFilled | Op::EachMatchFilledMany | Op::EachMatchMade => {
            let Some(asked) = EachMatch::key(frame, word) else {
                return Flow::Back;
            };
            let rows = keyed(frame, asked.site, asked.start, asked.from);
            frame.state.key.truncate(asked.start.0);
            frame.state.rows = rows;
            EachMatch::run(frame, word, at)
        }
        Op::ReadNode | Op::ReadFilled | Op::ReadMade => Read::run(frame, word, at),
        Op::Test
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants => Test::run(frame, word, at),
        Op::Let => LetValue::run(frame, word, at),
        Op::Emit => Emit::run(frame, word, at),
        Op::Exists => {
            exists(frame, word);
            Exists::run(frame, word, at)
        }
        Op::Or => {
            staged(frame, word);
            Or::run(frame, word, at)
        }
        Op::Fork => Fork::run(frame, word, at),
        Op::Axis => {
            axis(frame, word);
            Axis::run(frame, word, at)
        }
        Op::Nearest => {
            nearest(frame, word);
            Nearest::run(frame, word, at)
        }
        Op::Latest => {
            latest(frame, word);
            Latest::run(frame, word, at)
        }
        Op::Closure => {
            closure(frame, word);
            Closure::run(frame, word, at)
        }
        Op::Yield => {
            if mode == RunMode::Collect {
                YieldRow::run(frame, word, at)
            } else {
                Flow::Back
            }
        }
        Op::EachRange
        | Op::Children
        | Op::ChildrenOfField
        | Op::Within
        | Op::NearestDerived
        | Op::Call
        | Op::Render
        | Op::StageScan
        | Op::StageProbe
        | Op::StageLatest
        | Op::StageRead
        | Op::StageKeep
        | Op::StageCall
        | Op::StageProduct
        | Op::StageUse
        | Op::StageRender
        | Op::StageWalk
        | Op::StageWrite => {
            frame.state.work.unbuilt =
                InstructionCount(frame.state.work.unbuilt.0.saturating_add(1));
            Flow::Back
        }
    }
}

/// Binds the next row of the iteration a cursor is of; answers where the loop goes on, or nowhere when it has no row left.
#[inline]
fn next(frame: &mut StepFrame<'_, '_>, cursor: &mut IterationCursor) -> Option<CodeEntry> {
    let word = word_at(frame, cursor.word);
    match word.op {
        Op::EachRowNode | Op::EachRowFilled | Op::EachRowMade => EachRow::next(frame, word, cursor),
        Op::EachMatchNode | Op::EachMatchFilled | Op::EachMatchFilledMany | Op::EachMatchMade => {
            EachMatch::next(frame, word, cursor)
        }
        Op::EachKind => EachKind::next(frame, word, cursor),
        Op::Or => Or::next(frame, word, cursor),
        Op::Fork => Fork::next(frame, word, cursor),
        Op::Axis => Axis::next(frame, word, cursor),
        Op::Latest => Latest::next(frame, word, cursor),
        Op::Closure => Closure::next(frame, word, cursor),
        Op::End
        | Op::EachRange
        | Op::Children
        | Op::ChildrenOfField
        | Op::Within
        | Op::Nearest
        | Op::NearestDerived
        | Op::ReadNode
        | Op::ReadFilled
        | Op::ReadMade
        | Op::Test
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants
        | Op::Call
        | Op::Render
        | Op::Let
        | Op::Exists
        | Op::Yield
        | Op::Emit
        | Op::StageScan
        | Op::StageProbe
        | Op::StageLatest
        | Op::StageRead
        | Op::StageKeep
        | Op::StageCall
        | Op::StageProduct
        | Op::StageUse
        | Op::StageRender
        | Op::StageWalk
        | Op::StageWrite => None,
    }
}

/// Runs the program at `entry` for what `mode` says, and answers how many rows it gave: the rows a rule's branch emitted,
/// the rows a collected body yielded, or one for a body asked whether it has a row, which its first row ends. Its cursors
/// lie above its caller's on the one stack and are gone when it answers, and so are the rows its `Or` words staged. In a
/// rule's branch the first cursor pushed is the anchor's: each row that cursor gives is a candidate.
pub(crate) fn run(
    frame: &mut StepFrame<'_, '_>,
    entry: CodeEntry,
    mode: RunMode,
) -> InstructionCount {
    run_counted::<Uncounted>(frame, entry, mode)
}

/// `run`, telling `C` of each word: the one loop, of which the plain and the measuring worker each hold an instance.
pub(crate) fn run_counted<C: WordCounter>(
    frame: &mut StepFrame<'_, '_>,
    entry: CodeEntry,
    mode: RunMode,
) -> InstructionCount {
    let (base, staged_base) = (frame.state.frames.len(), frame.state.staged.len());
    let (mut at, mut rows, mut steps, mut visited, mut candidates) =
        (entry, 0_u64, 0_u64, 0_u64, 0_u64);
    'program: loop {
        let word = word_at(frame, at);
        C::word(frame, at);
        steps = steps.saturating_add(1);
        let gives = match mode {
            RunMode::Rows => word.op == Op::Emit,
            RunMode::Collect => word.op == Op::Yield,
            RunMode::Exists => word.op == Op::Yield || word.op == Op::End,
        };
        rows = rows.saturating_add(u64::from(gives));
        if gives && mode == RunMode::Exists {
            break 'program;
        }
        match step(frame, word, at, mode) {
            Flow::Next => at = after(at),
            Flow::Back => loop {
                let depth = frame.state.frames.len();
                let Some(mut cursor) = frame.state.frames.last().copied().filter(|_| depth > base)
                else {
                    break 'program;
                };
                if let Some(onward) = next(frame, &mut cursor) {
                    if let Some(kept) = frame.state.frames.get_mut(depth.saturating_sub(1)) {
                        *kept = cursor;
                    }
                    visited = visited.saturating_add(1);
                    candidates =
                        candidates.saturating_add(u64::from(depth == base.saturating_add(1)));
                    at = onward;
                    break;
                }
                frame.state.frames.truncate(depth.saturating_sub(1));
            },
        }
    }
    frame.state.frames.truncate(base);
    frame.state.staged.truncate(staged_base);
    let work = &mut frame.state.work;
    work.steps = InstructionCount(work.steps.0.saturating_add(steps));
    work.visited = InstructionCount(work.visited.0.saturating_add(visited));
    if mode == RunMode::Rows {
        work.candidates = InstructionCount(work.candidates.0.saturating_add(candidates));
    }
    let budget = &mut frame.state.budget;
    budget.visited = InstructionCount(budget.visited.0.saturating_add(visited));
    if mode == RunMode::Rows {
        budget.candidates = InstructionCount(budget.candidates.0.saturating_add(candidates));
    }
    InstructionCount(rows)
}
