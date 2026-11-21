//! The round trips of the `filling` codes: each constructor read back by its reader, with a different number in every slot, so a
//! constructor and a reader that disagree on a slot, or a constructor that writes another code's operation, fail their row.

use super::super::fixtures::{RoundTrip, agreed, round_trip};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::anchoring::types::LatestWord;
use crate::domain::code::deciding::types::{CallWord, RenderWord, TestWord};
use crate::domain::code::filling::service::{
    stage_call, stage_call_of, stage_keep, stage_keep_of, stage_latest, stage_latest_of,
    stage_probe, stage_probe_of, stage_product, stage_product_of, stage_read, stage_read_of,
    stage_render, stage_render_of, stage_scan, stage_scan_of, stage_use, stage_use_of, stage_walk,
    stage_walk_of, stage_write, stage_write_of,
};
use crate::domain::code::filling::types::{
    ProbeWord, ProductWord, ScanWord, StageReadWord, UseWord, WalkWord, WriteWord,
};
use crate::domain::code::types::{IndexSite, MemoSite, Op, PoolAt, Short, WordFlag, WordSlot};
use crate::domain::steps::types::{MadeTableId, RegexId, Register};
use ryo_language::domain::node_table::types::FieldCode;

type Input = RoundTrip;
/// The operation of the word whose reader gave back the operands written.
type Value = Op;
/// The operands written and the operands read back, when they differ.
type Failure = String;

/// The rows of this part's cases: one for each code.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("stage_scan_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ScanWord {
                    flag: WordFlag(0x85),
                    out: Register(0x0001_0001),
                    constants: PoolAt(0x0002_0002),
                    read: WordSlot(0x0003_0003),
                },
                stage_scan,
                stage_scan_of,
            )
        },
        expect: Expect::Value(Op::StageScan),
    },
    Case {
        name: CaseName("stage_probe_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ProbeWord {
                    flag: WordFlag(0x85),
                    values: Short(0x1234),
                    out: Register(0x0001_0001),
                    probed: WordSlot(0x0002_0002),
                    index: IndexSite(0x0003_0003),
                },
                stage_probe,
                stage_probe_of,
            )
        },
        expect: Expect::Value(Op::StageProbe),
    },
    Case {
        name: CaseName("stage_product_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                ProductWord {
                    flag: WordFlag(0x85),
                    out: Register(0x0001_0001),
                    read: WordSlot(0x0003_0003),
                },
                stage_product,
                stage_product_of,
            )
        },
        expect: Expect::Value(Op::StageProduct),
    },
    Case {
        name: CaseName("stage_use_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                UseWord {
                    flag: WordFlag(0x85),
                    outputs: Short(0x1234),
                    body: PoolAt(0x0001_0001),
                    registers: PoolAt(0x0002_0002),
                    memo: MemoSite(0x0003_0003),
                },
                stage_use,
                stage_use_of,
            )
        },
        expect: Expect::Value(Op::StageUse),
    },
    Case {
        name: CaseName("stage_latest_reads_back_what_it_wrote"),
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
                stage_latest,
                stage_latest_of,
            )
        },
        expect: Expect::Value(Op::StageLatest),
    },
    Case {
        name: CaseName("stage_read_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                StageReadWord {
                    flag: WordFlag(0x85),
                    column: Short(0x1234),
                    row: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                    read: WordSlot(0x0003_0003),
                },
                stage_read,
                stage_read_of,
            )
        },
        expect: Expect::Value(Op::StageRead),
    },
    Case {
        name: CaseName("stage_keep_reads_back_what_it_wrote"),
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
                stage_keep,
                stage_keep_of,
            )
        },
        expect: Expect::Value(Op::StageKeep),
    },
    Case {
        name: CaseName("stage_call_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                CallWord {
                    flag: WordFlag(0x0f),
                    meaning: Short(0x1234),
                    arguments: PoolAt(0x0001_0001),
                    outputs: PoolAt(0x0002_0002),
                    regex: RegexId(0x0003_0003),
                },
                stage_call,
                stage_call_of,
            )
        },
        expect: Expect::Value(Op::StageCall),
    },
    Case {
        name: CaseName("stage_render_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                RenderWord {
                    context: Short(0x1234),
                    node: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                stage_render,
                stage_render_of,
            )
        },
        expect: Expect::Value(Op::StageRender),
    },
    Case {
        name: CaseName("stage_walk_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                WalkWord {
                    flag: WordFlag(0x85),
                    field: FieldCode(0x1234),
                    start: Register(0x0001_0001),
                    out: Register(0x0002_0002),
                },
                stage_walk,
                stage_walk_of,
            )
        },
        expect: Expect::Value(Op::StageWalk),
    },
    Case {
        name: CaseName("stage_write_reads_back_what_it_wrote"),
        word: Word::Succeeds,
        input: || {
            round_trip(
                WriteWord {
                    flag: WordFlag(0x85),
                    columns: Short(0x1234),
                    table: MadeTableId(0x0001_0001),
                    operands: PoolAt(0x0003_0003),
                },
                stage_write,
                stage_write_of,
            )
        },
        expect: Expect::Value(Op::StageWrite),
    },
];

fn subject(trip: Input) -> Result<Value, Failure> {
    agreed(trip)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
