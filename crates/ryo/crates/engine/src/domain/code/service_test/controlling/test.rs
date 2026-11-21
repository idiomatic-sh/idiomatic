//! The round trips of the `controlling` codes: each constructor read back by its reader, with a different number in every slot, so a
//! constructor and a reader that disagree on a slot, or a constructor that writes another code's operation, fail their row.

use super::super::fixtures::{RoundTrip, agreed, round_trip};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::controlling::service::{
    emit, emit_of, end, exists, exists_of, fork, fork_of, or, or_of, yield_row, yield_row_of,
};
use crate::domain::code::controlling::types::{EmitWord, ExistsWord, ForkWord, OrWord, YieldWord};
use crate::domain::code::types::{CodeWord, MemoSite, Op, PoolAt, Short, WordFlag, WordSlot};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

type Input = RoundTrip;
/// The operation of the word whose reader gave back the operands written.
type Value = Op;
/// The operands written and the operands read back, when they differ.
type Failure = String;

/// The rows of this part's cases: one for each code.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("end_holds_nothing_but_its_operation"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                CodeWord {
                    op: Op::End,
                    ..CodeWord::default()
                },
                |_| end(),
                |word| word,
            )
        },
        expect: Expect::Value(Op::End),
    },
    Case {
        name: CaseName("exists_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ExistsWord {
                    flag: WordFlag(0x85),
                    keys: Short(0x1234),
                    body: PoolAt(0x0001_0001),
                    key: PoolAt(0x0002_0002),
                    memo: MemoSite(0x0003_0003),
                },
                exists,
                exists_of,
            )
        },
        expect: Expect::Value(Op::Exists),
    },
    Case {
        name: CaseName("or_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                OrWord {
                    flag: WordFlag(0x85),
                    bodies: Short(0x1234),
                    records: PoolAt(0x0001_0001),
                    outputs: PoolAt(0x0002_0002),
                    memo: MemoSite(0x0003_0003),
                },
                or,
                or_of,
            )
        },
        expect: Expect::Value(Op::Or),
    },
    Case {
        name: CaseName("fork_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ForkWord {
                    slots: Short(0x1234),
                    entries: PoolAt(0x0001_0001),
                },
                fork,
                fork_of,
            )
        },
        expect: Expect::Value(Op::Fork),
    },
    Case {
        name: CaseName("yield_row_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                YieldWord {
                    cells: Short(0x1234),
                    row: WordSlot(0x0001_0001),
                },
                yield_row,
                yield_row_of,
            )
        },
        expect: Expect::Value(Op::Yield),
    },
    Case {
        name: CaseName("emit_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EmitWord {
                    flag: WordFlag(0x85),
                    columns: Short(0x1234),
                    rule: SelectedRuleOrdinal(0x0001_0001),
                    input: WordSlot(0x0002_0002),
                    operands: PoolAt(0x0003_0003),
                },
                emit,
                emit_of,
            )
        },
        expect: Expect::Value(Op::Emit),
    },
];

fn subject(trip: Input) -> Result<Value, Failure> {
    agreed(trip)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
