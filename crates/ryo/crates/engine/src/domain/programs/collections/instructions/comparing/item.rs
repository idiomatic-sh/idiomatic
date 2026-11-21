//! `Test`: the candidate goes on when a comparison holds between two cells, a cell is in a set of constants, or a test
//! of a text holds (a regular expression, a glob). Each meaning is the one a `Keep` stage runs over a batch: the
//! comparison of two cells is `stages::service::cells_hold`, a test of a text is `stages::testing::service::keep_text`
//! for a batch of one row. A test of a text that kernel does not run yet ends the candidate and is counted.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::constants::{LEFT_CONSTANT, NEGATED, RIGHT_CONSTANT, RIGHT_SET};
use crate::domain::code::deciding::service::{
    test_equal_constant_of, test_equal_register_of, test_in_constants_of,
    test_not_equal_constant_of, test_not_equal_register_of, test_of,
};
use crate::domain::code::small_codes::service::comparison_of;
use crate::domain::code::types::{CodeEntry, CodeWord, ConstantNumber, Op, PoolAt, WordSlot};
use crate::domain::findings::types::InstructionCount;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, constant, listed};
use crate::domain::scratch::kept::service::holds_file;
use crate::domain::scratch::kept::types::RegisterHolds;
use crate::domain::scratch::types::LoopState;
use crate::domain::stages::service::{cell_test, cells_hold};
use crate::domain::stages::testing::service::keep_text;
use crate::domain::stages::types::{BatchValue, CellTest, RowKept, TextMeans, TextTest};
use crate::domain::steps::types::{Register, TestOperation};
use ryo_language::domain::facts::service::intern;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::terms::types::NameText;

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

/// Whether a comparison of two cells holds: the one definition of each, which a `Keep` stage's loop runs over a batch.
const fn compared(test: CellTest, left: CellWord, right: CellWord) -> Verdict {
    match cells_hold(test, left, right) {
        RowKept::Yes => Verdict::Holds,
        RowKept::No => Verdict::Fails,
    }
}

/// The left of a test of a text: its cell, and the register it was read from when it was.
#[derive(Clone, Copy, Debug)]
struct TextLeft {
    /// The cell.
    cell: CellWord,
    /// The register; none for a constant.
    register: Option<Register>,
}

/// What a test of a text says of one candidate: the cells are words of the file, and what the test holds for is the
/// meaning a `Keep` stage runs over a batch, asked here for a batch of one row. The right side is one cell, or the
/// constants of a set of the pool. A test of a text the kernel does not run is not run here either.
fn text(
    frame: &mut StepFrame<'_, '_>,
    test: TestOperation,
    left: TextLeft,
    right: Option<CellWord>,
    set: Option<PoolAt>,
) -> Verdict {
    let code = frame.fill.programs.code;
    // The file's text is its path: a register that holds the file is read as the word the path is, not as the word
    // numbered like the file (the fill side's rule for a pipeline's test, 2121c643a).
    let left = match left.register {
        Some(register)
            if holds_file(&frame.state.decoded.files, register) == RegisterHolds::TheFile =>
        {
            intern(frame.fill.words, NameText(frame.fill.path.0))
        }
        Some(_) | None => left.cell,
    };
    let start = frame.state.key.len();
    for number in set.map(|list| listed(&code.pool, list)).unwrap_or_default() {
        let held = constant(frame, ConstantNumber(number.0));
        frame.state.key.push(held);
    }
    let LoopState {
        key,
        patterns,
        kept,
        ..
    } = &mut *frame.state;
    let answer = keep_text(
        TextMeans {
            matcher: frame.fill.matcher,
            regexes: frame.fill.programs.regexes,
            compiled: patterns,
            verdicts: &mut frame.fill.store.verdicts,
            stamp: frame.fill.store.stamp,
        },
        &*frame.fill.words,
        TextTest {
            test,
            left: BatchValue::Constant(left),
            right: right.map(BatchValue::Constant),
            set: key.get(start..).unwrap_or_default(),
            set_at: set,
        },
        Count(1),
        kept,
    );
    key.truncate(start);
    match answer {
        Ok(()) if kept.is_empty() => Verdict::Fails,
        Ok(()) => Verdict::Holds,
        Err(_) => Verdict::Unbuilt,
    }
}

/// Whether a cell is in a sorted list of constants of the pool, or with `wanted` fails is not.
fn within(frame: &StepFrame<'_, '_>, value: CellWord, list: PoolAt, wanted: Verdict) -> Verdict {
    let constants = listed(&frame.fill.programs.code.pool, list);
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
fn general(frame: &mut StepFrame<'_, '_>, word: CodeWord) -> Verdict {
    let read = test_of(word);
    let left = if read.flag.0 & LEFT_CONSTANT.0 == 0 {
        cell(frame, Register(read.left.0))
    } else {
        constant(frame, ConstantNumber(read.left.0))
    };
    let Some(operation) = comparison_of(read.comparison, read.regex) else {
        return Verdict::Unbuilt;
    };
    let text_left = TextLeft {
        cell: left,
        register: (read.flag.0 & LEFT_CONSTANT.0 == 0).then_some(Register(read.left.0)),
    };
    if read.flag.0 & RIGHT_SET.0 != 0 {
        let set = PoolAt(read.right.0);
        return match operation {
            TestOperation::In => within(frame, left, set, Verdict::Holds),
            TestOperation::NotIn => within(frame, left, set, Verdict::Fails),
            TestOperation::Glob | TestOperation::NotGlob => {
                text(frame, operation, text_left, None, Some(set))
            }
            TestOperation::Equal
            | TestOperation::NotEqual
            | TestOperation::Less
            | TestOperation::LessOrEqual
            | TestOperation::Greater
            | TestOperation::GreaterOrEqual
            | TestOperation::Matches(_)
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
    match (cell_test(operation), operation) {
        (Some(cells), _) => compared(cells, left, right),
        (None, TestOperation::In | TestOperation::NotIn) => Verdict::Unbuilt,
        (
            None,
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
            | TestOperation::Successor,
        ) => text(frame, operation, text_left, Some(right), None),
    }
}

impl InstructionsItem for Test {
    #[inline(always)]
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
