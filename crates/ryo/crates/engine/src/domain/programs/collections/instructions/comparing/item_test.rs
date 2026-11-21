//! The `Test` words' cases: each comparison of two cells, an order as of whole numbers, a set of constants either way, the
//! specialised words, a regular expression and a glob asked of one candidate's text, and a test of a text the kernel does
//! not run, which is counted and ends the candidate.

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
use crate::domain::code::small_codes::service::{comparison_code, comparison_regex};
use crate::domain::code::types::{
    CodeEntry, CodeWord, ConstantNumber, PoolAt, ProgramCode, WordFlag, WordSlot,
};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{RegexId, Register, StepRegex, TestOperation};
use ryo_language::domain::facts::service::{file_words, intern};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::terms::types::NameText;

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
    /// The general word over texts: the left text, a word of the file, and the right side. Regular expression 0 is
    /// `^ab+$`.
    Text(TestOperation, &'static str, Right),
}

/// What the right side of a test of a text is.
#[derive(Clone, Copy, Debug)]
enum Right {
    /// The text in register 1, a word of the file.
    Register(&'static str),
    /// A set of two texts, constants whose words are the rule set's.
    Set([&'static str; 2]),
}

/// The word, the left cell and the right cell (both nought for a test of texts).
type Input = (Asked, u64, u64);
/// Whether the candidate goes on (1) or ends (0), how many words were counted as not run, and how many cells of the key
/// room are left.
type Value = (u8, u64, usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_cells_are_equal"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Equal), 5, 5),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("different_cells_are_not_equal"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Equal), 5, 6),
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName("not_equal_holds_for_different_cells"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::NotEqual), 5, 6),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("less_is_strict"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Less), 5, 5),
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName("an_order_is_of_whole_numbers_so_minus_one_is_below_nought"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Less), u64::MAX, 0),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("less_or_equal_holds_at_the_bound"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::LessOrEqual), 5, 5),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("greater_is_strict"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::Greater), 5, 5),
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName("greater_or_equal_holds_at_the_bound"),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::GreaterOrEqual), 5, 5),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("the_right_operand_can_be_a_constant"),
        word: Word::Succeeds,
        input: || (Asked::Constant(TestOperation::Greater), 10, 0),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("a_cell_of_the_set_is_in_it"),
        word: Word::Succeeds,
        input: || (Asked::Set(TestOperation::In), 9, 0),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("a_cell_of_the_set_is_not_out_of_it"),
        word: Word::Succeeds,
        input: || (Asked::Set(TestOperation::NotIn), 9, 0),
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName("a_cell_outside_the_set_is_out_of_it"),
        word: Word::Succeeds,
        input: || (Asked::Set(TestOperation::NotIn), 4, 0),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("the_word_for_a_constant_compares_with_it"),
        word: Word::Succeeds,
        input: || (Asked::EqualConstant, 9, 0),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("the_word_for_two_registers_compares_them"),
        word: Word::Succeeds,
        input: || (Asked::NotEqualRegister, 4, 4),
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName("the_word_for_a_set_finds_its_cell"),
        word: Word::Succeeds,
        input: || (Asked::InConstants(WordFlag(0)), 3, 0),
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("the_negated_word_for_a_set_refuses_its_cell"),
        word: Word::Succeeds,
        input: || (Asked::InConstants(NEGATED), 3, 0),
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName(
            "a_test_of_a_text_the_kernel_does_not_run_is_counted_and_ends_the_candidate",
        ),
        word: Word::Succeeds,
        input: || (Asked::Registers(TestOperation::StartsWith), 5, 5),
        expect: Expect::Value((0, 1, 0)),
    },
    Case {
        name: CaseName("a_text_its_expression_matches_goes_on"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::Matches(RegexId(0)),
                    "abbb",
                    Right::Register(""),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("a_text_its_expression_does_not_match_ends"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::Matches(RegexId(0)),
                    "abc",
                    Right::Register(""),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName("an_expression_the_rule_set_does_not_hold_is_counted"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::Matches(RegexId(4)),
                    "abbb",
                    Right::Register(""),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((0, 1, 0)),
    },
    Case {
        name: CaseName("a_text_its_glob_matches_goes_on"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::Glob,
                    "test_views.py",
                    Right::Register("test_*.py"),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("a_text_its_glob_does_not_match_ends"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::Glob,
                    "views.py",
                    Right::Register("test_*.py"),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((0, 0, 0)),
    },
    Case {
        name: CaseName("the_negated_glob_goes_on_when_the_glob_does_not_match"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::NotGlob,
                    "views.py",
                    Right::Register("test_*.py"),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("a_text_any_glob_of_a_set_matches_goes_on"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::Glob,
                    "conftest.py",
                    Right::Set(["test_*.py", "conf*.py"]),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((1, 0, 0)),
    },
    Case {
        name: CaseName("a_text_no_glob_of_a_set_matches_ends"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::Text(
                    TestOperation::Glob,
                    "views.py",
                    Right::Set(["test_*.py", "conf*.py"]),
                ),
                0,
                0,
            )
        },
        expect: Expect::Value((0, 0, 0)),
    },
];

/// The general word of an operation with a flag and a right slot.
fn general(operation: TestOperation, flag: WordFlag, right: u32) -> CodeWord {
    test(TestWord {
        flag,
        comparison: comparison_code(operation),
        left: WordSlot(0),
        right: WordSlot(right),
        regex: comparison_regex(operation).map_or(RegexId(0), |regex| regex),
    })
}

fn subject((asked, left, right): Input) -> Result<Value, Failure> {
    let mut programs = ProgramsEngine {
        regexes: vec![StepRegex("^ab+$".into())],
        ..ProgramsEngine::default()
    };
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
        Asked::Text(operation, _, Right::Register(_)) => general(operation, WordFlag(0), 1),
        Asked::Text(operation, _, Right::Set(texts)) => {
            // A constant of a set is a word: the two texts are interned first in the file's words, so their words are
            // the first two of a file of this rule set, whose base has none.
            let mut words = file_words(&programs.words);
            code.constants = texts
                .iter()
                .map(|each| intern(&mut words, NameText(each)))
                .collect();
            general(operation, RIGHT_SET, set.0)
        }
    };
    programs.code = code;
    let mut held = held(2);
    held.state.registers[0] = CellWord(left);
    held.state.registers[1] = CellWord(right);
    let (flow, _found) = framed(&programs, &mut held, &Idle, |frame| {
        if let Asked::Text(_, left, right) = asked {
            if let Right::Set(texts) = right {
                for each in texts {
                    let _word = intern(frame.fill.words, NameText(each));
                }
            }
            let cells = [
                intern(frame.fill.words, NameText(left)),
                match right {
                    Right::Register(text) => intern(frame.fill.words, NameText(text)),
                    Right::Set(_) => CellWord(0),
                },
            ];
            frame.state.registers[..2].copy_from_slice(&cells);
        }
        Test::run(frame, word, CodeEntry(1))
    });
    Ok((
        u8::from(flow == Flow::Next),
        held.state.work.unbuilt.0,
        held.state.key.len(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
