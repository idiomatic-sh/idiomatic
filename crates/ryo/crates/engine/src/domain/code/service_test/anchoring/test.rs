//! The round trips of the `anchoring` codes: each constructor read back by its reader, with a different number in every slot, so a
//! constructor and a reader that disagree on a slot, or a constructor that writes another code's operation, fail their row.

use super::super::fixtures::{RoundTrip, agreed, round_trip};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::anchoring::service::{
    each_kind, each_kind_of, each_match_filled, each_match_filled_many, each_match_filled_many_of,
    each_match_filled_of, each_match_made, each_match_made_of, each_match_node, each_match_node_of,
    each_range, each_range_of, each_row_filled, each_row_filled_of, each_row_made,
    each_row_made_of, each_row_node, each_row_node_of, latest, latest_of,
};
use crate::domain::code::anchoring::types::{
    EachKindWord, EachMatchMadeWord, EachMatchManyWord, EachMatchWord, EachRangeWord,
    EachRowFilledWord, EachRowMadeWord, EachRowNodeWord, LatestWord,
};
use crate::domain::code::types::{
    IndexSite, Op, PoolAt, RelationNumber, Short, WordFlag, WordSlot,
};
use crate::domain::steps::types::{MadeTableId, Register};
use ryo_relations::domain::relations::types::KindBits;

type Input = RoundTrip;
/// The operation of the word whose reader gave back the operands written.
type Value = Op;
/// The operands written and the operands read back, when they differ.
type Failure = String;

/// The rows of this part's cases: one for each code.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_kind_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachKindWord {
                    flag: WordFlag(0x85),
                    kind: KindBits(0x1234),
                    out: Register(0x0001_0001),
                    kinds: PoolAt(0x0003_0003),
                },
                each_kind,
                each_kind_of,
            )
        },
        expect: Expect::Value(Op::EachKind),
    },
    Case {
        name: CaseName("each_match_node_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachMatchWord {
                    flag: WordFlag(0x85),
                    out: Register(0x0001_0001),
                    value: WordSlot(0x0002_0002),
                    index: IndexSite(0x0003_0003),
                },
                each_match_node,
                each_match_node_of,
            )
        },
        expect: Expect::Value(Op::EachMatchNode),
    },
    Case {
        name: CaseName("each_match_filled_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachMatchWord {
                    flag: WordFlag(0x85),
                    out: Register(0x0001_0001),
                    value: WordSlot(0x0002_0002),
                    index: IndexSite(0x0003_0003),
                },
                each_match_filled,
                each_match_filled_of,
            )
        },
        expect: Expect::Value(Op::EachMatchFilled),
    },
    Case {
        name: CaseName("each_match_filled_many_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachMatchManyWord {
                    values: Short(0x1234),
                    out: Register(0x0001_0001),
                    operands: PoolAt(0x0002_0002),
                    index: IndexSite(0x0003_0003),
                },
                each_match_filled_many,
                each_match_filled_many_of,
            )
        },
        expect: Expect::Value(Op::EachMatchFilledMany),
    },
    Case {
        name: CaseName("each_match_made_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachMatchMadeWord {
                    flag: WordFlag(0x85),
                    values: Short(0x1234),
                    out: Register(0x0001_0001),
                    value: WordSlot(0x0002_0002),
                    index: IndexSite(0x0003_0003),
                },
                each_match_made,
                each_match_made_of,
            )
        },
        expect: Expect::Value(Op::EachMatchMade),
    },
    Case {
        name: CaseName("each_range_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachRangeWord {
                    flag: WordFlag(0x85),
                    values: Short(0x1234),
                    out: Register(0x0001_0001),
                    operands: PoolAt(0x0002_0002),
                    index: IndexSite(0x0003_0003),
                },
                each_range,
                each_range_of,
            )
        },
        expect: Expect::Value(Op::EachRange),
    },
    Case {
        name: CaseName("each_row_node_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachRowNodeWord {
                    flag: WordFlag(0x85),
                    out: Register(0x0001_0001),
                },
                each_row_node,
                each_row_node_of,
            )
        },
        expect: Expect::Value(Op::EachRowNode),
    },
    Case {
        name: CaseName("each_row_filled_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachRowFilledWord {
                    out: Register(0x0001_0001),
                    relation: RelationNumber(0x0003_0003),
                },
                each_row_filled,
                each_row_filled_of,
            )
        },
        expect: Expect::Value(Op::EachRowFilled),
    },
    Case {
        name: CaseName("each_row_made_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                EachRowMadeWord {
                    flag: WordFlag(0x85),
                    out: Register(0x0001_0001),
                    table: MadeTableId(0x0003_0003),
                },
                each_row_made,
                each_row_made_of,
            )
        },
        expect: Expect::Value(Op::EachRowMade),
    },
    Case {
        name: CaseName("latest_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                LatestWord {
                    flag: WordFlag(0x85),
                    keys: Short(0x1234),
                    out: Register(0x0001_0001),
                    bound: WordSlot(0x0002_0002),
                    operands: PoolAt(0x0003_0003),
                },
                latest,
                latest_of,
            )
        },
        expect: Expect::Value(Op::Latest),
    },
];

fn subject(trip: Input) -> Result<Value, Failure> {
    agreed(trip)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
