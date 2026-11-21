//! `EachKind`: every element of one kind, or of each of several kinds in the word's order, bound as its handle. The node
//! table lists its elements by kind; the word's flag says whether the file's own tree alone is read. The cursor holds the
//! place in the kind's list and, beside it, which of the kinds it is at.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::anchoring::service::each_kind_of;
use crate::domain::code::constants::{OWN_TREE, SEVERAL};
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{after, listed, node_tag, set};
use crate::domain::scratch::types::{FramePlace, IterationCursor};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{ids_of_kind, own_end};
use ryo_relations::domain::relations::types::{KindBits, Position};

/// The `EachKind` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EachKind;

impl InstructionsItem for EachKind {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        frame.state.frames.push(IterationCursor {
            word: at,
            ..IterationCursor::default()
        });
        Flow::Back
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        let read = each_kind_of(word);
        let nodes = frame.fill.nodes;
        let reach = if read.flag.0 & OWN_TREE.0 == 0 {
            Position(u32::MAX)
        } else {
            own_end(nodes)
        };
        loop {
            let which = usize::try_from(cursor.beside.0).unwrap_or(usize::MAX);
            let kind = if read.flag.0 & SEVERAL.0 == 0 {
                (which == 0).then_some(read.kind)?
            } else {
                let held = listed(&frame.fill.programs.code.pool, read.kinds).get(which)?;
                KindBits(u16::try_from(held.0).unwrap_or(u16::MAX))
            };
            let place = usize::try_from(cursor.at.0).unwrap_or(usize::MAX);
            let element = ids_of_kind(nodes, kind)
                .get(place)
                .copied()
                .filter(|element| element.0 < reach.0);
            if let Some(element) = element {
                let handle = CellWord(node_tag(frame).0 | u64::from(element.0));
                set(frame, read.out, handle);
                cursor.at = FramePlace(cursor.at.0.saturating_add(1));
                return Some(after(cursor.word));
            }
            cursor.beside = FramePlace(cursor.beside.0.saturating_add(1));
            cursor.at = FramePlace(0);
        }
    }
}
