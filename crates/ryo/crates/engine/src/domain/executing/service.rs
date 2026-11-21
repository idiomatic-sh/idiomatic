//! The services of the `executing` concept: the one loop over a program's words. A word is dispatched to its item; a word
//! that iterates pushes a cursor, and each time a binding is done the innermost cursor binds its next row. No word
//! allocates, and the loop reads no clock: steps and the rows its iterations gave are counted in locals and added to the
//! work once, when the program ends.

use crate::domain::code::types::{CodeEntry, CodeWord, Op};
use crate::domain::findings::types::InstructionCount;
use crate::domain::programs::collections::instructions::each_row::item::EachRow;
use crate::domain::programs::collections::instructions::emit::item::Emit;
use crate::domain::programs::collections::instructions::interface::{
    Flow, InstructionsItem, StepFrame,
};
use crate::domain::programs::collections::instructions::let_value::item::LetValue;
use crate::domain::programs::collections::instructions::read::item::Read;
use crate::domain::programs::collections::instructions::test::item::Test;
use crate::domain::programs::operands::service::after;
use crate::domain::scratch::types::IterationCursor;

/// The word at a place; the word that ends a program past the code's end.
#[inline]
fn word_at(frame: &StepFrame<'_, '_>, at: CodeEntry) -> CodeWord {
    match frame
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
fn step(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) -> Flow {
    match word.op {
        Op::End => Flow::Back,
        Op::EachRowNode | Op::EachRowFilled | Op::EachRowMade => EachRow::run(frame, word, at),
        Op::ReadNode | Op::ReadFilled | Op::ReadMade => Read::run(frame, word, at),
        Op::Test
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants => Test::run(frame, word, at),
        Op::Let => LetValue::run(frame, word, at),
        Op::Emit => Emit::run(frame, word, at),
        Op::EachKind
        | Op::EachMatchNode
        | Op::EachMatchFilled
        | Op::EachMatchFilledMany
        | Op::EachMatchMade
        | Op::EachRange
        | Op::Children
        | Op::ChildrenOfField
        | Op::Axis
        | Op::Within
        | Op::Nearest
        | Op::NearestDerived
        | Op::Closure
        | Op::Call
        | Op::Render
        | Op::Exists
        | Op::Or
        | Op::Fork
        | Op::Yield
        | Op::Latest
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
        Op::End
        | Op::EachKind
        | Op::EachMatchNode
        | Op::EachMatchFilled
        | Op::EachMatchFilledMany
        | Op::EachMatchMade
        | Op::EachRange
        | Op::Children
        | Op::ChildrenOfField
        | Op::Axis
        | Op::Within
        | Op::Nearest
        | Op::NearestDerived
        | Op::Closure
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
        | Op::Or
        | Op::Fork
        | Op::Yield
        | Op::Latest
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

/// Runs a rule's branch from `entry` to its end and answers how many rows it emitted. Its cursors lie above its caller's
/// on the one stack and are gone when it answers. The first cursor it pushes is its anchor's: each row that cursor gives
/// is a candidate.
pub(crate) fn run(frame: &mut StepFrame<'_, '_>, entry: CodeEntry) -> InstructionCount {
    let base = frame.state.frames.len();
    let (mut at, mut rows, mut steps, mut visited, mut candidates) =
        (entry, 0_u64, 0_u64, 0_u64, 0_u64);
    'program: loop {
        let word = word_at(frame, at);
        steps = steps.saturating_add(1);
        rows = rows.saturating_add(u64::from(word.op == Op::Emit));
        match step(frame, word, at) {
            Flow::Next => at = after(at),
            Flow::Back => loop {
                let depth = frame.state.frames.len();
                let Some(mut cursor) = frame.state.frames.last().copied().filter(|_| depth > base)
                else {
                    break 'program;
                };
                if let Some(onward) = next(frame, &mut cursor) {
                    if let Some(kept) = frame.state.frames.last_mut() {
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
    let work = &mut frame.state.work;
    work.steps = InstructionCount(work.steps.0.saturating_add(steps));
    work.visited = InstructionCount(work.visited.0.saturating_add(visited));
    work.candidates = InstructionCount(work.candidates.0.saturating_add(candidates));
    InstructionCount(rows)
}
