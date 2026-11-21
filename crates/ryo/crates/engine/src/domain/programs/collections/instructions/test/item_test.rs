//! The `Test` words' cases: each comparison of two cells, an order as of whole numbers, a set of constants either way, the
//! specialised words, and a comparison of texts, which is counted and ends the candidate.

use super::item::Test;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::constants::{NEGATED, RIGHT_CONSTANT, RIGHT_SET};
use crate::domain::code::deciding::service::{
    test, test_equal_constant, test_in_constants, test_not_equal_register,
};
use crate::domain::code::deciding::types::{
    ConstantTestWord, RegisterTestWord, SetTestWord, TestWord,
};
use crate::domain::code::records::service::push_list;
use crate::domain::code::small_codes::service::comparison_code;
use crate::domain::code::types::{
    CodeEntry, CodeWord, ConstantNumber, PoolAt, ProgramCode, WordFlag, WordSlot,
};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{framed, state};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{RegexId, Register, TestOperation};
use ryo_language::domain::facts::types::CellWord;

/// The word a case runs. Register 0 holds the left cell and register 1 the right; constant 0 is 3 and constant 1 is 9, and
/// the set is both.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The general word over two registers.
    Registers(TestOperation),
    /// The general word against constant 1.
    Constant(TestOperation),
    /// The general word against the set.
    Set(TestOperation),
    /// `TestEqualConstant` against constant 1.
    EqualConstant,
    /// `TestNotEqualRegister`.
    NotEqualRegister,
    /// `TestInConstants`, negated or not.
    InConstants(WordFlag),
}

/// The word, the left cell and the right cell.
type Input = (Asked, u64, u64);
/// Whether the candidate goes on (1) or ends (0), and how many words were counted as not run.
type Value = (u8, u64);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_cells_are_equal"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Equal), 5, 5),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("different_cells_are_not_equal"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Equal), 5, 6),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("not_equal_holds_for_different_cells"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::NotEqual), 5, 6),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("less_is_strict"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Less), 5, 5),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("an_order_is_of_whole_numbers_so_minus_one_is_below_nought"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Less), u64::MAX, 0),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("less_or_equal_holds_at_the_bound"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::LessOrEqual), 5, 5),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("greater_is_strict"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Greater), 5, 5),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("greater_or_equal_holds_at_the_bound"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::GreaterOrEqual), 5, 5),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("the_right_operand_can_be_a_constant"),
        word: Word::Succeeds,
        input: || (Asked::Constant(TestOperation::Greater), 10, 0),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_cell_of_the_set_is_in_it"),
        word: Word::Succeeds,
        input: || (Asked::Set(TestOperation::In), 9, 0),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_cell_of_the_set_is_not_out_of_it"),
        word: Word::Succeeds,
        input: || (Asked::Set(TestOperation::NotIn), 9, 0),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_cell_outside_the_set_is_out_of_it"),
        word: Word::Succeeds,
        input: || (Asked::Set(TestOperation::NotIn), 4, 0),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("the_word_for_a_constant_compares_with_it"),
        word: Word::Succeeds,
        input: || (Asked::EqualConstant, 9, 0),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("the_word_for_two_registers_compares_them"),
        word: Word::Succeeds,
        input: || (Asked::NotEqualRegister, 4, 4),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("the_word_for_a_set_finds_its_cell"),
        word: Word::Succeeds,
        input: || (Asked::InConstants(WordFlag(0)), 3, 0),
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("the_negated_word_for_a_set_refuses_its_cell"),
        word: Word::Succeeds,
        input: || (Asked::InConstants(NEGATED), 3, 0),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_comparison_of_texts_is_counted_and_ends_the_candidate"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Glob), 5, 5),
        expect: Expect::Value((0, 1)),
    },
];

/// The general word of an operation with a flag and a right slot.
fn general(operation: TestOperation, flag: WordFlag, right: u32) -> CodeWord {
    test(TestWord {
        flag,
        comparison: comparison_code(operation),
        left: WordSlot(0),
        right: WordSlot(right),
        regex: RegexId(0),
    })
}

fn subject((asked, left, right): Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        constants: vec![CellWord(3), CellWord(9)],
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let set: PoolAt = push_list(&mut code.pool, &[WordSlot(0), WordSlot(1)]);
    let word = match asked {
        Asked::Registers(operation) => general(operation, WordFlag(0), 1),
        Asked::Constant(operation) => general(operation, RIGHT_CONSTANT, 1),
        Asked::Set(operation) => general(operation, RIGHT_SET, set.0),
        Asked::EqualConstant => test_equal_constant(ConstantTestWord {
            register: Register(0),
            constant: ConstantNumber(1),
        }),
        Asked::NotEqualRegister => test_not_equal_register(RegisterTestWord {
            left: Register(0),
            right: Register(1),
        }),
        Asked::InConstants(flag) => test_in_constants(SetTestWord {
            flag,
            register: Register(0),
            constants: set,
        }),
    };
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = state(2);
    held.registers[0] = CellWord(left);
    held.registers[1] = CellWord(right);
    let (flow, _found) = framed(&programs, &mut held, |frame| {
        Test::run(frame, word, CodeEntry(1))
    });
    Ok((u8::from(flow == Flow::Next), held.work.unbuilt.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
