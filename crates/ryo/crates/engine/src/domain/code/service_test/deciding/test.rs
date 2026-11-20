//! The round trips of the `deciding` codes: each constructor read back by its reader, with a different number in every slot, so a
//! constructor and a reader that disagree on a slot, or a constructor that writes another code's operation, fail their row.

use super::super::fixtures::{RoundTrip, agreed, round_trip};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::deciding::service::{
    call, call_of, let_value, let_value_of, read_filled, read_filled_of, read_made, read_made_of,
    read_node, read_node_of, render, render_of, test, test_equal_constant, test_equal_constant_of,
    test_equal_register, test_equal_register_of, test_in_constants, test_in_constants_of,
    test_not_equal_constant, test_not_equal_constant_of, test_not_equal_register,
    test_not_equal_register_of, test_of,
};
use crate::domain::code::deciding::types::{
    CallWord, ConstantTestWord, LetWord, ReadFilledWord, ReadMadeWord, ReadNodeWord,
    RegisterTestWord, RenderWord, SetTestWord, TestWord,
};
use crate::domain::code::types::{
    ConstantNumber, Op, PoolAt, RelationNumber, Short, WordFlag, WordSlot,
};
use crate::domain::steps::types::{MadeColumn, MadeTableId, RegexId, Register};

type Input = RoundTrip;
/// The operation of the word whose reader gave back the operands written.
type Value = Op;
/// The operands written and the operands read back, when they differ.
type Failure = String;

/// The rows of this part's cases: one for each code.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("read_node_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ReadNodeWord {
                    column: Short(0x1234),
                    row: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                read_node,
                read_node_of,
            )
        },
        expect: Expect::Value(Op::ReadNode),
    },
    Case {
        name: CaseName("read_filled_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ReadFilledWord {
                    place: Short(0x1234),
                    row: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                    relation: RelationNumber(0x0003_0003),
                },
                read_filled,
                read_filled_of,
            )
        },
        expect: Expect::Value(Op::ReadFilled),
    },
    Case {
        name: CaseName("read_made_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ReadMadeWord {
                    column: MadeColumn(0x1234),
                    row: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                    table: MadeTableId(0x0003_0003),
                },
                read_made,
                read_made_of,
            )
        },
        expect: Expect::Value(Op::ReadMade),
    },
    Case {
        name: CaseName("test_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                TestWord {
                    flag: WordFlag(0x85),
                    comparison: Short(0x1234),
                    left: WordSlot(0x0001_0001),
                    right: WordSlot(0x0002_0002),
                    regex: RegexId(0x0003_0003),
                },
                test,
                test_of,
            )
        },
        expect: Expect::Value(Op::Test),
    },
    Case {
        name: CaseName("test_equal_constant_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ConstantTestWord {
                    register: Register(0x0001_0001),
                    constant: ConstantNumber(0x0002_0002),
                },
                test_equal_constant,
                test_equal_constant_of,
            )
        },
        expect: Expect::Value(Op::TestEqualConstant),
    },
    Case {
        name: CaseName("test_not_equal_constant_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ConstantTestWord {
                    register: Register(0x0001_0001),
                    constant: ConstantNumber(0x0002_0002),
                },
                test_not_equal_constant,
                test_not_equal_constant_of,
            )
        },
        expect: Expect::Value(Op::TestNotEqualConstant),
    },
    Case {
        name: CaseName("test_equal_register_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                RegisterTestWord {
                    left: Register(0x0001_0001),
                    right: Register(0x0002_0002),
                },
                test_equal_register,
                test_equal_register_of,
            )
        },
        expect: Expect::Value(Op::TestEqualRegister),
    },
    Case {
        name: CaseName("test_not_equal_register_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                RegisterTestWord {
                    left: Register(0x0001_0001),
                    right: Register(0x0002_0002),
                },
                test_not_equal_register,
                test_not_equal_register_of,
            )
        },
        expect: Expect::Value(Op::TestNotEqualRegister),
    },
    Case {
        name: CaseName("test_in_constants_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                SetTestWord {
                    flag: WordFlag(0x85),
                    register: Register(0x0001_0001),
                    constants: PoolAt(0x0002_0002),
                },
                test_in_constants,
                test_in_constants_of,
            )
        },
        expect: Expect::Value(Op::TestInConstants),
    },
    Case {
        name: CaseName("call_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                CallWord {
                    meaning: Short(0x1234),
                    arguments: PoolAt(0x0001_0001),
                    outputs: PoolAt(0x0002_0002),
                    regex: RegexId(0x0003_0003),
                },
                call,
                call_of,
            )
        },
        expect: Expect::Value(Op::Call),
    },
    Case {
        name: CaseName("render_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                RenderWord {
                    context: Short(0x1234),
                    node: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                render,
                render_of,
            )
        },
        expect: Expect::Value(Op::Render),
    },
    Case {
        name: CaseName("let_value_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                LetWord {
                    flag: WordFlag(0x85),
                    from: WordSlot(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                let_value,
                let_value_of,
            )
        },
        expect: Expect::Value(Op::Let),
    },
];

fn subject(trip: Input) -> Result<Value, Failure> {
    agreed(trip)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
