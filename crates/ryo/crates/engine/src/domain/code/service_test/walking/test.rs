//! The round trips of the `walking` codes: each constructor read back by its reader, with a different number in every slot, so a
//! constructor and a reader that disagree on a slot, or a constructor that writes another code's operation, fail their row.

use super::super::fixtures::{RoundTrip, agreed, round_trip};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{Op, PoolAt, WordFlag};
use crate::domain::code::walking::service::{
    axis, axis_of, children, children_of, children_of_field, children_of_field_of, closure,
    closure_of, nearest, nearest_derived, nearest_derived_of, nearest_of, within, within_of,
};
use crate::domain::code::walking::types::{
    AxisWord, ChildrenOfFieldWord, ChildrenWord, ClosureWord, NearestWord, WithinWord,
};
use crate::domain::steps::types::Register;
use ryo_language::domain::node_table::types::FieldCode;

type Input = RoundTrip;
/// The operation of the word whose reader gave back the operands written.
type Value = Op;
/// The operands written and the operands read back, when they differ.
type Failure = String;

/// The rows of this part's cases: one for each code.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("children_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ChildrenWord {
                    flag: WordFlag(0x80),
                    parent: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                children,
                children_of,
            )
        },
        expect: Expect::Value(Op::Children),
    },
    Case {
        name: CaseName("children_of_field_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ChildrenOfFieldWord {
                    flag: WordFlag(0x80),
                    field: FieldCode(0x1234),
                    parent: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                children_of_field,
                children_of_field_of,
            )
        },
        expect: Expect::Value(Op::ChildrenOfField),
    },
    Case {
        name: CaseName("axis_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                AxisWord {
                    flag: WordFlag(0x80),
                    direction: WordFlag(0x04),
                    start: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                axis,
                axis_of,
            )
        },
        expect: Expect::Value(Op::Axis),
    },
    Case {
        name: CaseName("within_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                WithinWord {
                    flag: WordFlag(0x80),
                    ancestor: Register(0x0001_0001),
                    descendant: Register(0x0002_0002),
                },
                within,
                within_of,
            )
        },
        expect: Expect::Value(Op::Within),
    },
    Case {
        name: CaseName("nearest_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                NearestWord {
                    flag: WordFlag(0x85),
                    start: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                    operands: PoolAt(0x0003_0003),
                },
                nearest,
                nearest_of,
            )
        },
        expect: Expect::Value(Op::Nearest),
    },
    Case {
        name: CaseName("nearest_derived_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                NearestWord {
                    flag: WordFlag(0x85),
                    start: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                    operands: PoolAt(0x0003_0003),
                },
                nearest_derived,
                nearest_derived_of,
            )
        },
        expect: Expect::Value(Op::NearestDerived),
    },
    Case {
        name: CaseName("closure_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ClosureWord {
                    seed: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                    operands: PoolAt(0x0003_0003),
                },
                closure,
                closure_of,
            )
        },
        expect: Expect::Value(Op::Closure),
    },
];

fn subject(trip: Input) -> Result<Value, Failure> {
    agreed(trip)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
