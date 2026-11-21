//! `EachMatch`: the rows of a source whose key columns hold the word's values, each bound as its handle. The loop asks the
//! fill side for the key's rows; this item puts the key together, and iterates the rows it is handed.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::anchoring::service::{
    each_match_filled_many_of, each_match_filled_of, each_match_made_of, each_match_node_of,
};
use crate::domain::code::constants::{CONSTANT, MANY};
use crate::domain::code::records::service::operand_at;
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::types::{
    CodeEntry, CodeWord, ConstantNumber, IndexSite, Op, PoolAt, WordFlag, WordSlot,
};
use crate::domain::indexes::types::RowRun;
use crate::domain::made_tables::reading::service::rows_at;
use crate::domain::made_tables::sources::service::handle_of;
use crate::domain::made_tables::types::KeyFrom;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{after, cell, constant, node_tag, operand, set};
use crate::domain::scratch::types::{FramePlace, IterationCursor};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The `EachMatch` words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EachMatch;

/// What a word asks its source: the index, where its key starts in the key room, and whether a cell of the key is read
/// from a register.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyAsked {
    /// The index of the source by the word's columns.
    pub(crate) site: IndexSite,
    /// Where the key starts in the loop's key room.
    pub(crate) start: Count,
    /// Whether the key comes from the candidate's row or from constants alone.
    pub(crate) from: KeyFrom,
}

/// What a word's one value is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueFrom {
    /// A register.
    Register,
    /// A constant, by number.
    Constant,
}

/// What a constant flag of a word says of its one value.
const fn value_from(flag: WordFlag) -> ValueFrom {
    if flag.0 & CONSTANT.0 == 0 {
        ValueFrom::Register
    } else {
        ValueFrom::Constant
    }
}

/// What a word's operands are: its index, its register out, and its values as one slot or as operands in the pool.
struct Operands {
    /// The index.
    site: IndexSite,
    /// The register each row's handle is written to.
    out: Register,
    /// How many values the pool holds for it; nought when the value is the one slot.
    many: WordSlot,
    /// The one value, or where the pool's operands start.
    value: WordSlot,
    /// What the one value is.
    one: ValueFrom,
}

/// A word's operands, by its code.
fn operands(word: CodeWord) -> Option<Operands> {
    Some(match word.op {
        Op::EachMatchNode | Op::EachMatchFilled => {
            let read = if word.op == Op::EachMatchNode {
                each_match_node_of(word)
            } else {
                each_match_filled_of(word)
            };
            Operands {
                site: read.index,
                out: read.out,
                many: WordSlot(0),
                value: read.value,
                one: value_from(read.flag),
            }
        }
        Op::EachMatchFilledMany => {
            let read = each_match_filled_many_of(word);
            Operands {
                site: read.index,
                out: read.out,
                many: WordSlot(u32::from(read.values.0)),
                value: WordSlot(read.operands.0),
                one: ValueFrom::Register,
            }
        }
        Op::EachMatchMade => {
            let read = each_match_made_of(word);
            let several = read.flag.0 & MANY.0 != 0;
            Operands {
                site: read.index,
                out: read.out,
                many: WordSlot(if several { u32::from(read.values.0) } else { 0 }),
                value: read.value,
                one: value_from(read.flag),
            }
        }
        Op::End
        | Op::EachKind
        | Op::EachRange
        | Op::EachRowNode
        | Op::EachRowFilled
        | Op::EachRowMade
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
        | Op::StageWrite => return None,
    })
}

impl EachMatch {
    /// Pushes the word's key onto the loop's key room, a cell for each value in the columns' order, and answers what the
    /// loop asks the fill side with. The loop takes the key off the room when it has asked.
    pub(crate) fn key(frame: &mut StepFrame<'_, '_>, word: CodeWord) -> Option<KeyAsked> {
        let read = operands(word)?;
        let start = Count(frame.state.key.len());
        let mut from = KeyFrom::Constants;
        if read.many.0 == 0 {
            let value = if read.one == ValueFrom::Constant {
                constant(frame, ConstantNumber(read.value.0))
            } else {
                from = KeyFrom::Row;
                cell(frame, Register(read.value.0))
            };
            frame.state.key.push(value);
        }
        for number in 0..read.many.0 {
            let at = PoolAt(read.value.0.wrapping_add(number.wrapping_mul(2)));
            let held = operand_at(&frame.fill.programs.code.pool, at);
            if matches!(held, Some(PooledOperand::Register(_))) {
                from = KeyFrom::Row;
            }
            let value = held.map_or(CellWord(0), |each| operand(frame, each));
            frame.state.key.push(value);
        }
        Some(KeyAsked {
            site: read.site,
            start,
            from,
        })
    }
}

impl InstructionsItem for EachMatch {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        let run = frame.state.rows.run;
        frame.state.frames.push(IterationCursor {
            word: at,
            at: FramePlace(u32::try_from(run.start.0).unwrap_or(u32::MAX)),
            end: FramePlace(u32::try_from(run.end.0).unwrap_or(u32::MAX)),
            beside: FramePlace(0),
        });
        Flow::Back
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        if cursor.at.0 >= cursor.end.0 {
            return None;
        }
        let place = usize::try_from(cursor.at.0).unwrap_or(usize::MAX);
        let row = *rows_at(
            frame.fill.store,
            RowRun {
                start: Count(place),
                end: Count(place.saturating_add(1)),
            },
        )
        .first()?;
        let tag = if word.op == Op::EachMatchNode {
            node_tag(frame)
        } else {
            CellWord(0)
        };
        set(frame, operands(word)?.out, handle_of(row, tag));
        cursor.at = FramePlace(cursor.at.0.saturating_add(1));
        Some(after(cursor.word))
    }
}
