//! The ordered read's cases, through the loop, over a fixture pack written as steps and encoded by the straight encoding.
//! The pack states a table of (key, order) rows, (1, 10), (1, 20), (1, 20), (2, 5) and (1, 40), which the fill side sorts
//! into a table read by `Latest`; the rule reads, for each (key, bound) of a table of probes, the rows nearest the bound
//! and emits the probe's key and bound and the row's order.

use super::service::latest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::Op;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::executing::service::run as run_branch;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, FillInput, LoweredPack, LoweredRule, MadeFill, OrderedFill,
};
use crate::domain::made_tables::fixtures::packs::{of, pipeline, table};
use crate::domain::programs::fixtures::{framed, held};
use crate::domain::programs::frames::types::RunMode;
use crate::domain::programs::operands::service::set;
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::types::{
    ColumnList, EmitTarget, MadeColumn, MadeTableId, MatchColumns, PlacedStep, ProgramStep,
    ReadColumn, Register, RegisterCount, StepOperand, StepOperands, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The side of the bound, and the (key, bound) probes, in order.
type Input = (OrderBound, &'static [[u64; 2]]);
/// The key, the bound and the order of each row found; then 99, the rows the run answered, the rows visited, the words
/// counted as not run, and how many cursors, staged cells and key cells are left; then, the word asked once more for the
/// first probe, the rows it answered and the cells it left staged.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("at_or_before_reads_every_row_tied_for_the_nearest_order"),
        word: Word::Succeeds,
        input: || (OrderBound::AtOrBefore, &[[1, 25], [1, 5], [2, 5], [3, 5]]),
        expect: Expect::Value(Cow::Borrowed(&[
            1, 25, 20, 1, 25, 20, 2, 5, 5, 99, 3, 7, 0, 0, 0, 0, 2, 2,
        ])),
    },
    Case {
        name: CaseName("before_leaves_out_the_bound_itself"),
        word: Word::Succeeds,
        input: || (OrderBound::Before, &[[1, 20], [2, 5]]),
        expect: Expect::Value(Cow::Borrowed(&[1, 20, 10, 99, 1, 3, 0, 0, 0, 0, 1, 1])),
    },
    Case {
        name: CaseName("at_or_after_reads_the_nearest_row_after"),
        word: Word::Succeeds,
        input: || (OrderBound::AtOrAfter, &[[1, 25], [1, 41]]),
        expect: Expect::Value(Cow::Borrowed(&[1, 25, 40, 99, 1, 3, 0, 0, 0, 0, 1, 1])),
    },
    Case {
        name: CaseName("after_leaves_out_the_bound_itself"),
        word: Word::Succeeds,
        input: || (OrderBound::After, &[[1, 10], [1, 40]]),
        expect: Expect::Value(Cow::Borrowed(&[
            1, 10, 20, 1, 10, 20, 99, 2, 4, 0, 0, 0, 0, 2, 2,
        ])),
    },
];

/// A table of constant rows of two cells.
fn pairs(node: u8, rows: &[[u64; 2]]) -> crate::domain::lowered::types::MadeTable {
    table(
        node,
        2,
        MadeFill::Constant(ConstantRows(
            rows.iter()
                .map(|row| -> Box<[CellWord]> { Box::new([CellWord(row[0]), CellWord(row[1])]) })
                .collect(),
        )),
    )
}

/// A read of a made table's column.
fn read(made: u32, column: u16, row: u32, out: u32) -> (StepRows, ReadColumn, Register, Register) {
    (
        StepRows::Made(MadeTableId(made)),
        ReadColumn::Made(MadeColumn(column)),
        Register(row),
        Register(out),
    )
}

/// The rule set: the stated rows (table 0), the sorted table over them (table 1), the probes (table 2), one rule.
fn pack((side, probes): Input) -> LoweredPack {
    let staged =
        |(source, column, row, out): (StepRows, ReadColumn, Register, Register)| FillStage::Read {
            source,
            column,
            row,
            out,
        };
    let stepped = |(source, column, row, out): (StepRows, ReadColumn, Register, Register)| {
        ProgramStep::Read {
            source,
            column,
            row,
            out,
        }
    };
    let sorted = table(
        92,
        2,
        MadeFill::Ordered(OrderedFill::Sorted(FillInput {
            pipelines: Box::new([pipeline(
                vec![
                    FillStage::Scan {
                        source: StepRows::Made(MadeTableId(0)),
                        columns: MatchColumns::default(),
                        values: StepOperands(Box::new([])),
                        out: Register(10),
                    },
                    staged(read(0, 0, 10, 11)),
                    staged(read(0, 1, 10, 12)),
                ],
                [of(11), of(12)],
            )]),
            keys: ColumnList(Box::new([MadeColumn(0)])),
            order: Some(MadeColumn(1)),
        })),
    );
    let steps = vec![
        ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(2)),
            out: Register(0),
        },
        stepped(read(2, 0, 0, 1)),
        stepped(read(2, 1, 0, 2)),
        ProgramStep::Latest {
            table: MadeTableId(1),
            key: StepOperands(Box::new([StepOperand::Register(Register(1))])),
            bound: StepOperand::Register(Register(2)),
            side,
            out: Register(3),
        },
        stepped(read(1, 1, 3, 4)),
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: StepOperands(Box::new([of(1), of(2), of(4)])),
        },
    ];
    LoweredPack {
        tables: vec![
            pairs(90, &[[1, 10], [1, 20], [1, 20], [2, 5], [1, 40]]),
            sorted,
            pairs(91, probes),
        ],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number, CellKind::Number, CellKind::Number]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(13),
        ..LoweredPack::default()
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let code = encoded(&pack(input)).map_err(|_| ())?;
    let entry = code
        .rules
        .first()
        .and_then(|rule| rule.branches.first())
        .map(|branch| branch.entry)
        .ok_or(())?;
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(13);
    let (rows, found) = framed(&programs, &mut held, &TheLoop, |frame| {
        run_branch(frame, entry, RunMode::Rows)
    });
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.extend([
        99,
        rows.0,
        held.state.work.visited.0,
        held.state.work.unbuilt.0,
        u64::try_from(held.state.frames.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.key.len()).unwrap_or(u64::MAX),
    ]);
    // Then the word asked once more, outside a run, for the first probe: what it leaves staged for its item.
    let first = input.1.first().copied().unwrap_or_default();
    let (asked, _found) = framed(&programs, &mut held, &TheLoop, |frame| {
        let reading = frame
            .fill
            .programs
            .code
            .words
            .iter()
            .copied()
            .find(|word| word.op == Op::Latest);
        set(frame, Register(1), CellWord(first[0]));
        set(frame, Register(2), CellWord(first[1]));
        if let Some(word) = reading {
            latest(frame, word);
        }
        [
            u64::try_from(frame.state.answer.rows.0).unwrap_or(u64::MAX),
            u64::try_from(frame.state.staged.len()).unwrap_or(u64::MAX),
        ]
    });
    all.extend(asked);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
