//! `Test`: the candidate goes on when a comparison holds between two cells, or a cell is in a set of constants. A
//! comparison of two cells is decided here; a comparison of texts (a pattern, a glob, a prefix) is the fill side's meaning
//! and is not built yet: the candidate then ends and the word is counted.

use super::super::interface::{Flow, InstructionsItem, StepFrame};
use crate::domain::code::constants::{LEFT_CONSTANT, NEGATED, RIGHT_CONSTANT, RIGHT_SET};
use crate::domain::code::deciding::service::{
    test_equal_constant_of, test_equal_register_of, test_in_constants_of,
    test_not_equal_constant_of, test_not_equal_register_of, test_of,
};
use crate::domain::code::small_codes::service::comparison_of;
use crate::domain::code::types::{CodeEntry, CodeWord, ConstantNumber, Op, PoolAt, WordSlot};
use crate::domain::findings::types::InstructionCount;
use crate::domain::programs::operands::service::{cell, constant, listed};
use crate::domain::stages::types::CellTest;
use crate::domain::steps::types::{Register, TestOperation};
use ryo_language::domain::facts::types::CellWord;

/// The `Test` words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Test;

/// What a test says of the candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The comparison holds.
    Holds,
    /// It does not.
    Fails,
    /// The comparison is of texts, which this item does not decide.
    Unbuilt,
}

/// The comparison of two cells a test operation is, when it is one: what a `Keep` stage's loop takes too.
pub(crate) const fn cell_test(test: TestOperation) -> Option<CellTest> {
    match test {
        TestOperation::Equal => Some(CellTest::Equal),
        TestOperation::NotEqual => Some(CellTest::NotEqual),
        TestOperation::Less => Some(CellTest::Less),
        TestOperation::LessOrEqual => Some(CellTest::LessOrEqual),
        TestOperation::Greater => Some(CellTest::Greater),
        TestOperation::GreaterOrEqual => Some(CellTest::GreaterOrEqual),
        TestOperation::In
        | TestOperation::NotIn
        | TestOperation::Matches(_)
        | TestOperation::Glob
        | TestOperation::NotGlob
        | TestOperation::StartsWith
        | TestOperation::EndsWith
        | TestOperation::NumericAtLeast
        | TestOperation::NaturalLess
        | TestOperation::Successor => None,
    }
}

/// Whether a comparison of two cells holds. An order is of whole numbers.
const fn compared(test: CellTest, left: CellWord, right: CellWord) -> Verdict {
    let holds = match test {
        CellTest::Equal => left.0 == right.0,
        CellTest::NotEqual => left.0 != right.0,
        CellTest::Less => left.0.cast_signed() < right.0.cast_signed(),
        CellTest::LessOrEqual => left.0.cast_signed() <= right.0.cast_signed(),
        CellTest::Greater => left.0.cast_signed() > right.0.cast_signed(),
        CellTest::GreaterOrEqual => left.0.cast_signed() >= right.0.cast_signed(),
    };
    if holds {
        Verdict::Holds
    } else {
        Verdict::Fails
    }
}

/// Whether a cell is in a sorted list of constants of the pool, or with `wanted` fails is not.
fn within(frame: &StepFrame<'_, '_>, value: CellWord, list: PoolAt, wanted: Verdict) -> Verdict {
    let constants = listed(&frame.programs.code.pool, list);
    let found = constants
        .binary_search_by(|number: &WordSlot| constant(frame, ConstantNumber(number.0)).cmp(&value))
        .is_ok();
    if found == (wanted == Verdict::Holds) {
        Verdict::Holds
    } else {
        Verdict::Fails
    }
}

/// What a word of the general test says: both operands by its flag, the comparison by its code.
fn general(frame: &StepFrame<'_, '_>, word: CodeWord) -> Verdict {
    let read = test_of(word);
    let left = if read.flag.0 & LEFT_CONSTANT.0 == 0 {
        cell(frame, Register(read.left.0))
    } else {
        constant(frame, ConstantNumber(read.left.0))
    };
    let Some(operation) = comparison_of(read.comparison, read.regex) else {
        return Verdict::Unbuilt;
    };
    if read.flag.0 & RIGHT_SET.0 != 0 {
        return match operation {
            TestOperation::In => within(frame, left, PoolAt(read.right.0), Verdict::Holds),
            TestOperation::NotIn => within(frame, left, PoolAt(read.right.0), Verdict::Fails),
            TestOperation::Equal
            | TestOperation::NotEqual
            | TestOperation::Less
            | TestOperation::LessOrEqual
            | TestOperation::Greater
            | TestOperation::GreaterOrEqual
            | TestOperation::Matches(_)
            | TestOperation::Glob
            | TestOperation::NotGlob
            | TestOperation::StartsWith
            | TestOperation::EndsWith
            | TestOperation::NumericAtLeast
            | TestOperation::NaturalLess
            | TestOperation::Successor => Verdict::Unbuilt,
        };
    }
    let right = if read.flag.0 & RIGHT_CONSTANT.0 == 0 {
        cell(frame, Register(read.right.0))
    } else {
        constant(frame, ConstantNumber(read.right.0))
    };
    match cell_test(operation) {
        Some(cells) => compared(cells, left, right),
        None => Verdict::Unbuilt,
    }
}

impl InstructionsItem for Test {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let verdict = match word.op {
            Op::TestEqualConstant => {
                let read = test_equal_constant_of(word);
                compared(
                    CellTest::Equal,
                    cell(frame, read.register),
                    constant(frame, read.constant),
                )
            }
            Op::TestNotEqualConstant => {
                let read = test_not_equal_constant_of(word);
                compared(
                    CellTest::NotEqual,
                    cell(frame, read.register),
                    constant(frame, read.constant),
                )
            }
            Op::TestEqualRegister => {
                let read = test_equal_register_of(word);
                compared(
                    CellTest::Equal,
                    cell(frame, read.left),
                    cell(frame, read.right),
                )
            }
            Op::TestNotEqualRegister => {
                let read = test_not_equal_register_of(word);
                compared(
                    CellTest::NotEqual,
                    cell(frame, read.left),
                    cell(frame, read.right),
                )
            }
            Op::TestInConstants => {
                let read = test_in_constants_of(word);
                let wanted = if read.flag.0 & NEGATED.0 == 0 {
                    Verdict::Holds
                } else {
                    Verdict::Fails
                };
                within(frame, cell(frame, read.register), read.constants, wanted)
            }
            Op::Test => general(frame, word),
            Op::End
            | Op::EachKind
            | Op::EachMatchNode
            | Op::EachMatchFilled
            | Op::EachMatchFilledMany
            | Op::EachMatchMade
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
            | Op::StageWrite => Verdict::Unbuilt,
        };
        match verdict {
            Verdict::Holds => Flow::Next,
            Verdict::Fails => Flow::Back,
            Verdict::Unbuilt => {
                frame.state.work.unbuilt =
                    InstructionCount(frame.state.work.unbuilt.0.saturating_add(1));
                Flow::Back
            }
        }
    }
}
