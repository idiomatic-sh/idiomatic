//! The services of the `walks` part: the elements a word reaches along a tree axis from one start, staged for the word's
//! item, the nearest of a start's ancestors its condition holds of, and whether one node is a proper ancestor of another. The walk itself is the fill side's kernel
//! (`stages::walking::service::along`), run for a batch of one start: the tree has one definition of a parent, of a
//! sibling and of the reach of an extent.

use crate::domain::code::constants::{KEPT, OWN_TREE};
use crate::domain::code::small_codes::service::direction_of;
use crate::domain::code::types::{CodeEntry, CodeWord, MemoSite, WordFlag, WordSlot};
use crate::domain::code::walking::service::{axis_of, nearest_of, within_of};
use crate::domain::executing::bodies::service::{body_of, has_row, slot};
use crate::domain::findings::types::InstructionCount;
use crate::domain::memos::service::{asked, keep, rows};
use crate::domain::memos::types::MemoId;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, node_tag, set};
use crate::domain::scratch::types::{BodyAnswer, LoopState};
use crate::domain::stages::types::FanCursor;
use crate::domain::stages::walking::service::along;
use crate::domain::steps::types::{AxisDirection, Register};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, NodeExtent};

/// The extent a walking word's flag names: the file's own tree, or the embedded trees with it.
const fn extent(flag: WordFlag) -> NodeExtent {
    if flag.0 & OWN_TREE.0 == 0 {
        NodeExtent::Embedded
    } else {
        NodeExtent::Own
    }
}

/// Lends the staged room to a kernel of the fill side that gives several rows for one: its fan writes after the cells
/// staged so far, every row of a batch of one, each handle with `tag`. Answers where the fan's cells start.
pub(crate) fn lent(state: &mut LoopState, tag: CellWord) -> Count {
    let LoopState { staged, fan, .. } = state;
    let first = staged.len();
    std::mem::swap(&mut fan.out, staged);
    fan.up.clear();
    fan.limit = Count(usize::MAX);
    fan.tag = tag;
    fan.cursor = FanCursor::default();
    Count(first)
}

/// Takes the staged room back from the fan; answers where the cells it wrote are.
pub(crate) fn taken_back(state: &mut LoopState, first: Count) -> BodyAnswer {
    let LoopState { staged, fan, .. } = state;
    std::mem::swap(&mut fan.out, staged);
    BodyAnswer {
        first,
        rows: Count(staged.len().saturating_sub(first.0)),
    }
}

/// Stages the handles of the elements `direction` reaches from `start`, in the walk's order; answers where they are.
fn reached(
    frame: &mut StepFrame<'_, '_>,
    extent: NodeExtent,
    direction: AxisDirection,
    start: CellWord,
) -> BodyAnswer {
    let tag = node_tag(frame);
    let first = lent(frame.state, tag);
    let _all = along(
        frame.fill.nodes,
        extent,
        direction,
        &[start],
        Count(1),
        &mut frame.state.fan,
    );
    taken_back(frame.state, first)
}

/// Stages what an `Axis` word reaches from its start, and leaves where in the state for the word's item. A direction the
/// code does not hold reaches nothing.
pub(crate) fn axis(frame: &mut StepFrame<'_, '_>, word: CodeWord) {
    let asking = axis_of(word);
    let start = cell(frame, asking.start);
    frame.state.answer = match direction_of(asking.direction) {
        Some(direction) => reached(frame, extent(asking.flag), direction, start),
        None => BodyAnswer {
            first: Count(frame.state.staged.len()),
            rows: Count(0),
        },
    };
}

/// The nearest ancestor of `start` the condition at the word's record holds of: each ancestor, nearest first, is written
/// to the record's element register and the condition is asked of it. Each ancestor asked is a row visited.
fn walked(
    frame: &mut StepFrame<'_, '_>,
    word: CodeWord,
    at: CodeEntry,
    start: CellWord,
) -> Option<CellWord> {
    let asking = nearest_of(word);
    let body = body_of(frame, at)?;
    let element = Register(slot(frame, body.after, WordSlot(0)).0);
    let ancestors = reached(frame, extent(asking.flag), AxisDirection::Ancestor, start);
    let (mut found, mut tried) = (None, 0_u64);
    for place in 0..ancestors.rows.0 {
        let Some(ancestor) = frame
            .state
            .staged
            .get(ancestors.first.0.saturating_add(place))
            .copied()
        else {
            break;
        };
        tried = tried.saturating_add(1);
        set(frame, element, ancestor);
        if has_row(frame, Some(body)).0 > 0 {
            found = Some(ancestor);
            break;
        }
    }
    frame.state.staged.truncate(ancestors.first.0);
    let LoopState { work, budget, .. } = &mut *frame.state;
    work.visited = InstructionCount(work.visited.0.saturating_add(tried));
    budget.visited = InstructionCount(budget.visited.0.saturating_add(tried));
    found
}

/// Answers a `Nearest` word over the tree: the nearest ancestor of its start its condition holds of, never the start;
/// kept per start for the file when the word says so. The element found is staged, and where is left in the state for
/// the word's item; nothing is staged when no ancestor holds.
pub(crate) fn nearest(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) {
    let asking = nearest_of(word);
    let start = cell(frame, asking.start);
    let found = if asking.flag.0 & KEPT.0 == 0 {
        walked(frame, word, at, start)
    } else {
        let site = body_of(frame, at).map_or(MemoSite(0), |body| {
            MemoSite(slot(frame, body.after, WordSlot(1)).0)
        });
        match asked(&frame.state.memos, site, &[start]) {
            Some(answer) => rows(&frame.state.memos, answer).first().copied(),
            None => {
                let found = walked(frame, word, at, start);
                let (cells, count) = match &found {
                    Some(held) => (std::slice::from_ref(held), Count(1)),
                    None => (&[][..], Count(0)),
                };
                let _kept: MemoId = keep(&mut frame.state.memos, site, &[start], cells, count);
                found
            }
        }
    };
    let first = frame.state.staged.len();
    frame.state.staged.extend(found);
    frame.state.answer = BodyAnswer {
        first: Count(first),
        rows: Count(usize::from(found.is_some())),
    };
}

/// Answers a `Within` word: whether its ancestor register's node is a proper ancestor of its descendant register's, in
/// the tree the word's flag names. The descendant's ancestors are the walk kernel's (the tree's one definition of a
/// parent and of an extent's reach), and each ancestor looked at is a row visited. Leaves one row in the state's answer
/// when it holds and none when it does not; nothing stays staged.
pub(crate) fn within(frame: &mut StepFrame<'_, '_>, word: CodeWord) {
    let asking = within_of(word);
    let (above, below) = (cell(frame, asking.ancestor), cell(frame, asking.descendant));
    let ancestors = reached(frame, extent(asking.flag), AxisDirection::Ancestor, below);
    let walked = frame
        .state
        .staged
        .get(ancestors.first.0..)
        .unwrap_or_default();
    let found = walked.iter().position(|each| *each == above);
    let looked = u64::try_from(found.map_or(walked.len(), |place| place.saturating_add(1)))
        .unwrap_or(u64::MAX);
    frame.state.staged.truncate(ancestors.first.0);
    let LoopState {
        work,
        budget,
        answer,
        ..
    } = &mut *frame.state;
    work.visited = InstructionCount(work.visited.0.saturating_add(looked));
    budget.visited = InstructionCount(budget.visited.0.saturating_add(looked));
    *answer = BodyAnswer {
        first: ancestors.first,
        rows: Count(usize::from(found.is_some())),
    };
}
